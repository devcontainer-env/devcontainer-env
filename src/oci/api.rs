#![allow(dead_code)]

use anyhow::Result;
use async_trait::async_trait;
use bollard::Docker;
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};
use url::Url;

#[cfg(test)]
use mockall::automock;

/// Parsed representation of a `devcontainer.json` configuration file.
#[derive(serde::Deserialize)]
pub struct Spec {
    /// Environment variables set inside the container at build/start time.
    #[serde(alias = "containerEnv")]
    pub container_env: Option<Environment>,
}

impl Spec {
    /// Reads and parses a `devcontainer.json` (JSONC) file at `path`.
    fn read_from_file(path: &Path) -> Result<Spec> {
        let data = String::from_utf8(std::fs::read(path)?)?;
        let spec = jsonc_parser::parse_to_serde_value(data.as_ref(), &Default::default())?;
        Ok(spec)
    }
}

/// Represents a resolved devcontainer workspace, including its containers and environment.
#[derive(Debug)]
pub struct Workspace {
    /// Absolute path to the workspace folder.
    pub folder: PathBuf,
    /// Absolute path to the devcontainer.json configuration file.
    pub config: PathBuf,
    /// Running containers associated with this workspace.
    pub containers: Vec<Container>,
    /// Environment variables exported by the devcontainer services.
    pub environment: Environment,
}

impl Workspace {
    /// Rewrites environment variable values that are URLs referencing a container host,
    /// replacing the container port with the corresponding mapped host port.
    fn rewrite(mut self) -> Self {
        for var in &mut self.environment.variables {
            if let Some(value) = Url::parse(&var.value).ok().and_then(|mut url| {
                let host = url.host()?.to_string();
                let port = url.port_or_known_default()?;
                let container = self.containers.iter().find(|c| c.hosts.contains(&host))?;
                let mapping = container
                    .ports
                    .iter()
                    .find(|m| m.container_port == port && m.host_port != 0)?;
                url.set_ip_host("127.0.0.1".parse().unwrap()).unwrap();
                url.set_port(Some(mapping.host_port)).ok()?;
                Some(url.to_string())
            }) {
                var.value = value;
            }
        }
        self
    }
}

impl std::fmt::Display for Workspace {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Workspace: {}", self.folder.display())?;

        if !self.containers.is_empty() {
            writeln!(f)?;
            writeln!(f, "Containers:")?;

            let mut iter = self.containers.iter().peekable();
            // We should print the containers separated by new line.
            while let Some(container) = iter.next() {
                write!(f, "{container}")?;
                // Do not write new line if we are the last container.
                if iter.peek().is_some() || !self.environment.variables.is_empty() {
                    writeln!(f)?;
                }
            }
            if !self.environment.variables.is_empty() {
                writeln!(f, "Environment:")?;
                for variable in &self.environment.variables {
                    writeln!(f, "  {variable}")?;
                }
            }
        }

        Ok(())
    }
}

/// Represents a running Docker container within the devcontainer workspace.
#[derive(Debug)]
pub struct Container {
    /// Unique container ID.
    pub id: String,
    /// Container names.
    pub names: Vec<String>,
    /// Image name used to create the container.
    pub image: String,
    /// Container host list.
    pub hosts: Vec<String>,
    /// Port mappings exposed by this container.
    pub ports: Vec<PortMapping>,
    /// Environment variables set in the container.
    pub environment: Environment,
}

impl std::fmt::Display for Container {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let names: Vec<String> = self
            .names
            .iter()
            .map(|n| n.trim_start_matches('/').to_string())
            .collect();
        writeln!(f, "  {}", names.join(", "))?;
        writeln!(f, "    Image: {}", self.image)?;
        if !self.hosts.is_empty() {
            writeln!(f, "    Hosts: {}", self.hosts.join(", "))?;
        }
        if !self.ports.is_empty() {
            let ports: Vec<String> = self.ports.iter().map(|p| p.to_string()).collect();
            writeln!(f, "    Ports: {}", ports.join(", "))?;
        }

        Ok(())
    }
}

impl From<bollard::plugin::ContainerSummary> for Container {
    fn from(value: bollard::plugin::ContainerSummary) -> Self {
        let mut container = Self {
            id: value.id.unwrap(),
            names: value.names.unwrap(),
            image: value.image.unwrap(),
            hosts: value
                .network_settings
                .unwrap_or_default()
                .networks
                .unwrap_or_default()
                .values()
                .flat_map(|endpoint| endpoint.dns_names.clone().unwrap_or_default())
                .collect(),
            ports: value
                .ports
                .unwrap_or_default()
                .iter()
                .map(|p| p.clone().into())
                .collect(),
            environment: Environment::default(),
        };

        if let Some(labels) = value.labels {
            let service = labels.get("com.docker.compose.service").cloned();
            // We consider the service name as the main host.
            if let Some(name) = service {
                if !container.hosts.contains(&name) {
                    container.hosts.insert(0, name);
                }
            }
        }

        container
    }
}

impl From<bollard::plugin::ContainerInspectResponse> for Container {
    fn from(value: bollard::plugin::ContainerInspectResponse) -> Self {
        let mut container = Self {
            id: value.id.unwrap_or_default(),

            names: value
                .name
                .into_iter()
                .map(|n| n.trim_start_matches('/').to_string())
                .collect(),

            image: value
                .config
                .as_ref()
                .and_then(|c| c.image.clone())
                .unwrap_or_default(),

            environment: value
                .config
                .as_ref()
                .and_then(|c| c.env.clone())
                .unwrap_or_default()
                .into(),

            hosts: value
                .network_settings
                .as_ref()
                .and_then(|ns| ns.networks.as_ref())
                .map(|nets| {
                    nets.values()
                        .flat_map(|endpoint| endpoint.dns_names.clone().unwrap_or_default())
                        .collect()
                })
                .unwrap_or_default(),

            ports: value
                .network_settings
                .as_ref()
                .and_then(|ns| ns.ports.as_ref())
                .map(|ports| {
                    ports
                        .iter()
                        .flat_map(|(key, bindings)| {
                            // key example: "80/tcp"
                            let (port_str, proto) =
                                key.split_once('/').unwrap_or((key.as_str(), "tcp"));

                            let container_port = port_str.parse::<u16>().unwrap_or_default();

                            bindings.as_ref().into_iter().flat_map(move |vec| {
                                vec.iter().map(move |b| PortMapping {
                                    container_port,
                                    protocol: proto.to_string(),
                                    host_ip: b.host_ip.clone().unwrap_or_default(),
                                    host_port: b
                                        .host_port
                                        .as_deref()
                                        .and_then(|p| p.parse().ok())
                                        .unwrap_or_default(),
                                })
                            })
                        })
                        .collect()
                })
                .unwrap_or_default(),
        };

        // Same compose service logic as before
        if let Some(labels) = value.config.as_ref().and_then(|c| c.labels.as_ref()) {
            if let Some(name) = labels.get("com.docker.compose.service") {
                if !container.hosts.contains(name) {
                    container.hosts.insert(0, name.clone());
                }
            }
        }

        container
    }
}

/// A single key/value label filter for Docker container queries.
pub struct ContainerFilter {
    /// Label key to filter on.
    pub name: String,
    /// Expected label value.
    pub value: String,
}

impl std::fmt::Display for ContainerFilter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}={}", self.name, self.value)
    }
}

/// A collection of [`ContainerFilter`]s that converts into [`bollard`] list options.
pub struct ListContainersFilter(pub Vec<ContainerFilter>);

impl From<ListContainersFilter> for bollard::query_parameters::ListContainersOptions {
    fn from(filters: ListContainersFilter) -> Self {
        let mut predicate: HashMap<String, Vec<String>> = HashMap::new();
        predicate.insert(
            "label".to_string(),
            filters.0.iter().map(|f| f.to_string()).collect(),
        );
        bollard::query_parameters::ListContainersOptionsBuilder::new()
            .filters(&predicate)
            .all(false)
            .build()
    }
}

/// A single environment variable as a key-value pair.
#[derive(Debug, Clone)]
pub struct Variable {
    /// The variable name.
    pub key: String,
    /// The variable value.
    pub value: String,
}

impl std::fmt::Display for Variable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} = {}", self.key, self.value)
    }
}

impl std::str::FromStr for Variable {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (key, value) = s.split_once('=').ok_or(())?;
        Ok(Self {
            key: key.to_string(),
            value: value.to_string(),
        })
    }
}

/// Resolved environment variables for a devcontainer workspace,
/// with optional host-port URL rewriting applied.
#[derive(Debug, Default, Clone, serde::Deserialize)]
#[serde(from = "HashMap<String, String>")]
pub struct Environment {
    pub variables: Vec<Variable>,
}

impl Environment {
    /// Overrides values in `self` with matching keys from `other`, ignoring undeclared keys.
    fn apply(mut self, other: &Environment) -> Self {
        for var in &mut self.variables {
            if let Some(runtime) = other.variables.iter().find(|v| v.key == var.key) {
                var.value = runtime.value.clone();
            }
        }
        self
    }
}

impl From<Vec<String>> for Environment {
    fn from(env: Vec<String>) -> Self {
        Self {
            variables: env.into_iter().filter_map(|e| e.parse().ok()).collect(),
        }
    }
}

impl From<HashMap<String, String>> for Environment {
    fn from(env: HashMap<String, String>) -> Self {
        Self {
            variables: env
                .into_iter()
                .map(|(key, value)| Variable { key, value })
                .collect(),
        }
    }
}

impl From<Environment> for HashMap<String, String> {
    fn from(env: Environment) -> Self {
        env.variables
            .into_iter()
            .map(|v| (v.key, v.value))
            .collect()
    }
}

/// Describes a single port mapping between a container port and a host port.
#[derive(Debug)]
pub struct PortMapping {
    /// Port number inside the container.
    pub container_port: u16,
    /// Corresponding IP address on the host.
    pub host_ip: String,
    /// Corresponding port number on the host.
    pub host_port: u16,
    /// Transport protocol (e.g. `"tcp"` or `"udp"`).
    pub protocol: String,
}

impl std::fmt::Display for PortMapping {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} → {}:{}",
            self.container_port, self.host_ip, self.host_port
        )
    }
}

impl From<bollard::plugin::PortSummary> for PortMapping {
    fn from(summary: bollard::plugin::PortSummary) -> Self {
        Self {
            host_ip: summary.ip.unwrap_or_default(),
            host_port: summary.public_port.unwrap_or_default(),
            container_port: summary.private_port,
            protocol: summary
                .typ
                .unwrap_or(bollard::plugin::PortSummaryTypeEnum::TCP)
                .to_string(),
        }
    }
}

/// Abstraction over the Docker daemon API, allowing the real client to be swapped for a mock in tests.
#[async_trait]
#[cfg_attr(test, automock)]
pub trait DockerClient {
    /// Inspects a container by name or ID, returning its full metadata.
    async fn inspect_container(
        &self,
        name: &str,
        options: Option<bollard::query_parameters::InspectContainerOptions>,
    ) -> Result<bollard::plugin::ContainerInspectResponse>;
    /// Lists containers matching the given filter options.
    async fn list_containers(
        &self,
        opts: Option<bollard::query_parameters::ListContainersOptions>,
    ) -> Result<Vec<bollard::plugin::ContainerSummary>>;
}

#[async_trait]
impl DockerClient for Docker {
    async fn inspect_container(
        &self,
        name: &str,
        opts: Option<bollard::query_parameters::InspectContainerOptions>,
    ) -> Result<bollard::plugin::ContainerInspectResponse> {
        Ok(Docker::inspect_container(self, name, opts).await?)
    }

    async fn list_containers(
        &self,
        opts: Option<bollard::query_parameters::ListContainersOptions>,
    ) -> Result<Vec<bollard::plugin::ContainerSummary>> {
        Ok(Docker::list_containers(self, opts).await?)
    }
}

/// Parameters for [`Client::get_workspace`].
pub struct GetWorkspaceParam {
    /// Path to the devcontainer.json configuration file.
    pub config: PathBuf,
    /// Path to the workspace root folder.
    pub folder: PathBuf,
}

/// High-level client for resolving a devcontainer workspace from its configuration.
#[async_trait]
#[cfg_attr(test, automock)]
pub trait WorkspaceClient {
    /// Resolves the workspace described by `args`, returning its containers and environment variables.
    async fn get_workspace(&self, args: &GetWorkspaceParam) -> Result<Workspace>;
}

/// Docker client for querying devcontainer workspace state via the OCI/Docker API.
pub struct Client<D: DockerClient> {
    client: D,
}

impl Client<Docker> {
    /// Creates a new [`Client`] connected via the default Docker socket.
    ///
    /// Returns an error if the Docker socket cannot be reached.
    pub fn new() -> Result<Self> {
        let client = Docker::connect_with_socket_defaults()?;
        Ok(Self { client })
    }
}

#[async_trait]
impl<D: DockerClient + Send + Sync> WorkspaceClient for Client<D> {
    /// Resolves and returns the [`Workspace`] described by `args`.
    ///
    /// Canonicalizes the workspace folder and config paths, then queries the
    /// Docker daemon for containers and environment variables.
    ///
    /// # Errors
    /// Returns an error if any path cannot be canonicalized or if the Docker
    /// daemon returns an error.
    async fn get_workspace(&self, args: &GetWorkspaceParam) -> Result<Workspace> {
        let folder = args.folder.canonicalize()?;
        let config = if args.config.is_relative() {
            folder.join(&args.config)
        } else {
            args.config.clone()
        };
        let config = config.canonicalize()?;

        let config_dir = config
            .parent()
            .ok_or_else(|| anyhow::anyhow!("config path has no parent directory"))?;

        let filters = vec![
            ContainerFilter {
                name: String::from("com.docker.compose.project.working_dir"),
                value: config_dir.display().to_string(),
            },
            ContainerFilter {
                name: String::from("devcontainer.local_folder"),
                value: folder.display().to_string(),
            },
        ];

        let mut containers: Vec<Container> = Vec::new();
        // Try each filter until we list the desired containers
        for filter in filters {
            let opts = Some(ListContainersFilter(vec![filter]).into());
            let collection = self.client.list_containers(opts).await?;

            for summary in collection {
                let id = summary.id.clone().unwrap_or_default();
                let inspect = self.client.inspect_container(&id, None).await?;
                containers.push(inspect.into());
            }

            if !containers.is_empty() {
                break;
            }
        }

        let environment = containers
            .first()
            .map(|c| c.environment.clone())
            .unwrap_or_default();

        let spec = Spec::read_from_file(&config)?;
        let environment = spec.container_env.unwrap_or_default().apply(&environment);

        Ok(Workspace {
            folder,
            config,
            containers,
            environment,
        }
        .rewrite())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;
    use std::sync::LazyLock;
    use tempfile::TempDir;

    static WORKSPACE_FOLDER: LazyLock<PathBuf> = LazyLock::new(|| {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .canonicalize()
            .expect("failed to canonicalize workspace path")
    });

    /// Creates a temporary devcontainer.json fixture for testing
    fn create_test_fixture() -> Result<(TempDir, PathBuf, PathBuf)> {
        let temp_dir = TempDir::new()?;
        let devcontainer_dir = temp_dir.path().join(".devcontainer");
        std::fs::create_dir_all(&devcontainer_dir)?;

        let config_file = devcontainer_dir.join("devcontainer.json");
        std::fs::write(&config_file, "{}")?;

        let workspace_folder = temp_dir.path().to_path_buf();

        Ok((temp_dir, workspace_folder, config_file))
    }

    fn container_summary() -> bollard::plugin::ContainerSummary {
        bollard::plugin::ContainerSummary {
            id: Some("1".to_string()),
            names: Some(vec!["devcontainer-app-1".to_string()]),
            image: Some("mcr.microsoft.com/devcontainers/rust:latest".to_string()),
            ports: Some(vec![bollard::plugin::PortSummary {
                ip: Some("127.0.0.1".to_string()),
                public_port: Some(8080),
                private_port: 8080,
                typ: Some(bollard::plugin::PortSummaryTypeEnum::TCP),
            }]),
            ..Default::default()
        }
    }

    fn container_inspect_response() -> bollard::plugin::ContainerInspectResponse {
        bollard::plugin::ContainerInspectResponse {
            id: Some("1".to_string()),
            name: Some("/devcontainer-app-1".to_string()),
            config: Some(bollard::plugin::ContainerConfig {
                image: Some("mcr.microsoft.com/devcontainers/rust:latest".to_string()),
                ..Default::default()
            }),
            network_settings: Some(bollard::plugin::NetworkSettings {
                ports: Some({
                    let mut ports = HashMap::new();
                    ports.insert(
                        "8080/tcp".to_string(),
                        Some(vec![bollard::plugin::PortBinding {
                            host_ip: Some("127.0.0.1".to_string()),
                            host_port: Some("8080".to_string()),
                        }]),
                    );
                    ports
                }),
                ..Default::default()
            }),
            ..Default::default()
        }
    }

    fn expect_list_containers(
        client: &mut MockDockerClient,
        res: Result<Vec<bollard::plugin::ContainerSummary>>,
    ) {
        client
            .expect_list_containers()
            .return_once(|_| Box::pin(async move { res }));
    }

    fn expect_inspect_container(
        client: &mut MockDockerClient,
        res: Result<bollard::plugin::ContainerInspectResponse>,
    ) {
        client
            .expect_inspect_container()
            .return_once(|_, _| Box::pin(async move { res }));
    }

    #[tokio::test]
    async fn get_workspace_returns_containers() -> Result<()> {
        let (_temp_dir, folder, config) = create_test_fixture()?;

        let mut client = MockDockerClient::new();
        expect_list_containers(&mut client, Ok(vec![container_summary()]));
        expect_inspect_container(&mut client, Ok(container_inspect_response()));
        let client = Client { client };

        let workspace = client
            .get_workspace(&GetWorkspaceParam {
                config,
                folder,
            })
            .await?;

        assert_eq!(workspace.containers.len(), 1);
        assert_eq!(workspace.containers[0].names, vec!["devcontainer-app-1"]);
        Ok(())
    }

    #[tokio::test]
    async fn get_workspace_fails_when_docker_errors() -> Result<()> {
        let (_temp_dir, folder, config) = create_test_fixture()?;

        let mut client = MockDockerClient::new();
        expect_list_containers(&mut client, Err(anyhow::anyhow!("oh no")));
        let client = Client { client };

        let result = client
            .get_workspace(&GetWorkspaceParam {
                config,
                folder,
            })
            .await;

        assert_eq!(result.unwrap_err().to_string(), "oh no");
        Ok(())
    }

    #[tokio::test]
    async fn workspace_displays_as_text() -> Result<()> {
        let (_temp_dir, folder, config) = create_test_fixture()?;

        let mut client = MockDockerClient::new();
        expect_list_containers(&mut client, Ok(vec![container_summary()]));
        expect_inspect_container(&mut client, Ok(container_inspect_response()));
        let client = Client { client };

        let workspace = client
            .get_workspace(&GetWorkspaceParam {
                config,
                folder,
            })
            .await?;

        let output = workspace.to_string();
        assert!(output.contains("Workspace:"));
        assert!(output.contains("Containers:"));
        assert!(output.contains("devcontainer-app-1"));
        assert!(output.contains("Image: mcr.microsoft.com/devcontainers/rust:latest"));
        assert!(output.contains("Ports: 8080 → 127.0.0.1:8080"));
        Ok(())
    }

    #[test]
    fn workspace_displays_without_environment() {
        let workspace = Workspace {
            folder: ".".into(),
            config: ".devcontainer/devcontainer.json".into(),
            containers: vec![Container {
                id: "1".to_string(),
                names: vec!["app".to_string()],
                image: "rust:latest".to_string(),
                hosts: vec![],
                ports: vec![],
                environment: Environment::default(),
            }],
            environment: Environment { variables: vec![] },
        };

        let output = workspace.to_string();
        assert!(output.contains("Containers:"));
        assert!(!output.contains("Environment:"));
    }

    #[test]
    fn environment_apply_overrides_with_runtime_value() {
        let env = Environment::from(HashMap::from([("VAR".to_string(), "default".to_string())]));
        let result = env.apply(&Environment::from(vec!["VAR=real_value".to_string()]));
        assert_eq!(result.variables[0].value, "real_value");
    }

    #[test]
    fn environment_apply_ignores_undeclared_keys() {
        let env = Environment::from(HashMap::from([("VAR".to_string(), "default".to_string())]));
        let result = env.apply(&Environment::from(vec![
            "VAR=allowed".to_string(),
            "IGNORED=ignored".to_string(),
        ]));
        assert_eq!(result.variables.len(), 1);
        assert_eq!(result.variables[0].value, "allowed");
    }

    #[test]
    fn environment_apply_keeps_default_when_key_absent() {
        let env = Environment::from(HashMap::from([("VAR".to_string(), "default".to_string())]));
        let result = env.apply(&Environment::from(vec![]));
        assert_eq!(result.variables[0].value, "default");
    }

    #[test]
    fn environment_from_vec_parses_key_value_pairs() {
        let env = Environment::from(vec!["FOO=bar".to_string(), "BAZ=qux".to_string()]);
        assert_eq!(env.variables.len(), 2);
    }

    #[test]
    fn container_from_summary() {
        let container = Container::from(container_summary());
        assert_eq!(container.id, "1");
        assert_eq!(container.names, vec!["devcontainer-app-1"]);
        assert_eq!(
            container.image,
            "mcr.microsoft.com/devcontainers/rust:latest"
        );
        assert_eq!(container.ports.len(), 1);
        assert_eq!(container.ports[0].container_port, 8080);
        assert_eq!(container.ports[0].host_port, 8080);
        assert_eq!(container.ports[0].host_ip, "127.0.0.1");
    }

    #[test]
    fn container_includes_compose_service_as_host() {
        let summary = bollard::plugin::ContainerSummary {
            id: Some("1".to_string()),
            names: Some(vec!["devcontainer-app-1".to_string()]),
            image: Some("rust:latest".to_string()),
            labels: Some(HashMap::from([(
                "com.docker.compose.service".to_string(),
                "app".to_string(),
            )])),
            ..Default::default()
        };

        let container = Container::from(summary);
        assert_eq!(container.hosts.first().map(String::as_str), Some("app"));
    }

    #[test]
    fn port_mapping_from_summary() {
        let summary = bollard::plugin::PortSummary {
            ip: Some("127.0.0.1".to_string()),
            public_port: Some(8080),
            private_port: 80,
            typ: Some(bollard::plugin::PortSummaryTypeEnum::TCP),
        };
        let port = PortMapping::from(summary);
        assert_eq!(port.host_ip, "127.0.0.1");
        assert_eq!(port.host_port, 8080);
        assert_eq!(port.container_port, 80);
        assert_eq!(
            port.protocol,
            bollard::plugin::PortSummaryTypeEnum::TCP.to_string()
        );
    }

    #[test]
    fn port_mapping_from_summary_uses_defaults_when_fields_absent() {
        let summary = bollard::plugin::PortSummary {
            ip: None,
            public_port: None,
            private_port: 80,
            typ: None,
        };
        let port = PortMapping::from(summary);
        assert_eq!(port.host_ip, "");
        assert_eq!(port.host_port, 0);
        assert_eq!(port.container_port, 80);
        assert_eq!(
            port.protocol,
            bollard::plugin::PortSummaryTypeEnum::TCP.to_string()
        );
    }

    #[test]
    fn port_mapping_display() {
        let port = PortMapping {
            container_port: 80,
            host_ip: "127.0.0.1".to_string(),
            host_port: 8080,
            protocol: "tcp".to_string(),
        };
        assert_eq!(port.to_string(), "80 → 127.0.0.1:8080");
    }

    // --- Workspace::rewrite tests ---

    #[test]
    fn workspace_rewrite_rewrites_url_env_vars() {
        let workspace = Workspace {
            folder: ".".into(),
            config: ".devcontainer/devcontainer.json".into(),
            containers: vec![Container {
                id: "1".to_string(),
                names: vec!["app".to_string()],
                image: "rust:latest".to_string(),
                hosts: vec!["app".to_string()],
                ports: vec![PortMapping {
                    container_port: 5432,
                    host_ip: "127.0.0.1".to_string(),
                    host_port: 54320,
                    protocol: "tcp".to_string(),
                }],
                environment: Environment::default(),
            }],
            environment: Environment::from(vec!["DB_URL=postgres://app:5432/db".to_string()]),
        };
        let rewritten = workspace.rewrite();
        assert_eq!(
            rewritten.environment.variables[0].value,
            "postgres://127.0.0.1:54320/db"
        );
    }

    #[test]
    fn workspace_rewrite_leaves_non_url_vars_unchanged() {
        let workspace = Workspace {
            folder: ".".into(),
            config: ".devcontainer/devcontainer.json".into(),
            containers: vec![],
            environment: Environment::from(vec!["FOO=bar".to_string()]),
        };
        let rewritten = workspace.rewrite();
        assert_eq!(rewritten.environment.variables[0].value, "bar");
    }

    #[test]
    fn workspace_rewrite_skips_url_when_port_not_published() {
        let workspace = Workspace {
            folder: ".".into(),
            config: ".devcontainer/devcontainer.json".into(),
            containers: vec![Container {
                id: "1".to_string(),
                names: vec!["db".to_string()],
                image: "postgres:latest".to_string(),
                hosts: vec!["db".to_string()],
                ports: vec![PortMapping {
                    container_port: 5432,
                    host_ip: "".to_string(),
                    host_port: 0, // not published to host
                    protocol: "tcp".to_string(),
                }],
                environment: Environment::default(),
            }],
            environment: Environment::from(vec!["DB_URL=postgres://db:5432/mydb".to_string()]),
        };
        let rewritten = workspace.rewrite();
        assert_eq!(
            rewritten.environment.variables[0].value,
            "postgres://db:5432/mydb" // unchanged: port is internal-only
        );
    }

    // --- Container::from(ContainerInspectResponse) tests ---

    #[test]
    fn container_from_inspect_response() {
        let response = bollard::plugin::ContainerInspectResponse {
            id: Some("abc123".to_string()),
            name: Some("/my-container".to_string()),
            config: Some(bollard::plugin::ContainerConfig {
                image: Some("rust:latest".to_string()),
                env: Some(vec!["FOO=bar".to_string(), "BAZ=qux".to_string()]),
                ..Default::default()
            }),
            ..Default::default()
        };
        let container = Container::from(response);
        assert_eq!(container.id, "abc123");
        assert_eq!(container.names, vec!["my-container"]);
        assert_eq!(container.image, "rust:latest");
        let keys: Vec<&str> = container
            .environment
            .variables
            .iter()
            .map(|v| v.key.as_str())
            .collect();
        let values: Vec<&str> = container
            .environment
            .variables
            .iter()
            .map(|v| v.value.as_str())
            .collect();
        assert_eq!(keys, vec!["FOO", "BAZ"]);
        assert_eq!(values, vec!["bar", "qux"]);
    }

    #[test]
    fn container_from_inspect_response_includes_compose_service_as_host() {
        let response = bollard::plugin::ContainerInspectResponse {
            id: Some("abc123".to_string()),
            name: Some("/my-container".to_string()),
            config: Some(bollard::plugin::ContainerConfig {
                image: Some("rust:latest".to_string()),
                labels: Some(HashMap::from([(
                    "com.docker.compose.service".to_string(),
                    "app".to_string(),
                )])),
                ..Default::default()
            }),
            ..Default::default()
        };
        let container = Container::from(response);
        assert_eq!(container.hosts.first().map(String::as_str), Some("app"));
    }

    // --- get_workspace failure path test ---

    #[tokio::test]
    async fn get_workspace_fails_when_inspect_errors() -> Result<()> {
        let (_temp_dir, folder, config) = create_test_fixture()?;

        let mut client = MockDockerClient::new();
        expect_list_containers(&mut client, Ok(vec![container_summary()]));
        expect_inspect_container(&mut client, Err(anyhow::anyhow!("inspect failed")));
        let client = Client { client };

        let result = client
            .get_workspace(&GetWorkspaceParam {
                config,
                folder,
            })
            .await;

        assert_eq!(result.unwrap_err().to_string(), "inspect failed");
        Ok(())
    }
}
