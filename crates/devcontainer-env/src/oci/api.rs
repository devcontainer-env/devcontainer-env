#![allow(dead_code)]

use anyhow::Result;
use async_trait::async_trait;
use bollard::Docker;
use std::{collections::HashMap, path::{Path, PathBuf}};

#[cfg(test)]
use mockall::automock;

/// Parsed representation of a `devcontainer.json` configuration file.
#[derive(serde::Deserialize)]
pub struct Spec {
    /// Environment variables injected into the remote user's shell.
    #[serde(alias = "remoteEnv")]
    pub remote_env: Option<HashMap<String, String>>,
    /// Environment variables set inside the container at build/start time.
    #[serde(alias = "containerEnv")]
    pub container_env: Option<HashMap<String, String>>,
}

impl Spec {
    /// Reads and parses a `devcontainer.json` (JSONC) file at `path`.
    fn read_from_file(path: &Path) -> Result<Spec> {
        let data = String::from_utf8(std::fs::read(path)?)?;
        let spec = jsonc_parser::parse_to_serde_value(data.as_ref(), &Default::default())?;
        Ok(spec)
    }

    /// Returns variables declared in `container_env`, using runtime `environment` values
    /// where available and falling back to the spec defaults for any absent keys.
    fn resolve_env(&self, environment: &Vec<String>) -> HashMap<String, String> {
        let mut variables = HashMap::new();

        if let Some(kv) = &self.container_env {
            // Seed with spec defaults so declared-but-absent keys are still included.
            for (key, default) in kv {
                variables.insert(key.clone(), default.clone());
            }
            // Override with actual runtime values where available.
            for entry in environment {
                if let Some((key, value)) = entry.split_once('=') {
                    if variables.contains_key(key) {
                        variables.insert(key.to_string(), value.to_string());
                    }
                }
            }
        }

        variables
    }
}

/// Represents a resolved devcontainer workspace, including its containers and environment.
#[derive(Debug)]
pub struct Workspace {
    /// Absolute path to the workspace folder.
    pub folder: PathBuf,
    /// Absolute path to the devcontainer.json configuration file.
    pub config: PathBuf,
    /// Environment variables exported by the devcontainer services.
    pub variables: Vec<Variable>,
    /// Running containers associated with this workspace.
    pub containers: Vec<Container>,
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
                if iter.peek().is_some() || !self.variables.is_empty() {
                    writeln!(f)?;
                }
            }
            if !self.variables.is_empty() {
                writeln!(f, "Environment:")?;
                for var in &self.variables {
                    writeln!(f, "  {var}")?;
                }
            }
        }

        Ok(())
    }
}

/// A single environment variable as a key-value pair.
#[derive(Debug)]
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

/// A newtype wrapper around a [`Vec<Variable>`] that supports conversion into a [`HashMap`].
pub struct VecVariable(pub Vec<Variable>);

impl From<VecVariable> for HashMap<String, String> {
    fn from(vars: VecVariable) -> Self {
        vars.0.into_iter().map(|var| (var.key, var.value)).collect()
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
    fn from(summary: bollard::plugin::ContainerSummary) -> Self {
        let mut container = Self {
            id: summary.id.unwrap(),
            names: summary.names.unwrap(),
            image: summary.image.unwrap(),
            hosts: summary
                .network_settings
                .unwrap_or_default()
                .networks
                .unwrap_or_default()
                .values()
                .flat_map(|endpoint| endpoint.dns_names.clone().unwrap_or_default())
                .collect(),
            ports: summary
                .ports
                .unwrap_or_default()
                .iter()
                .map(|p| p.clone().into())
                .collect(),
        };

        if let Some(labels) = summary.labels {
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
            // Prepare the options
            let opts = Some(ListContainersFilter(vec![filter]).into());
            // List the main containers
            let collection = self.client.list_containers(opts).await?;
            // Transform the collection
            for item in collection {
                containers.push(item.into());
            }
            // We should stop
            if !containers.is_empty() {
                break;
            }
        }

        let mut variables = Vec::new();
        let spec: Spec = Spec::read_from_file(&config)?;

        // Let's prepare the variables
        if let Some(container) = containers.first() {
            let metadata = self.client.inspect_container(&container.id, None).await?;
            let environment = metadata.config.unwrap_or_default().env.unwrap_or_default();
            let environment = spec.resolve_env(&environment);
            // Transform the variables into structs
            for (key, value) in environment {
                variables.push(Variable {
                    key: key.to_string(),
                    value: value.to_string(),
                })
            }
        }

        Ok(Workspace {
            folder,
            config,
            variables,
            containers,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;
    use std::sync::LazyLock;

    static WORKSPACE_FOLDER: LazyLock<PathBuf> = LazyLock::new(|| {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .canonicalize()
            .expect("failed to canonicalize workspace path")
    });

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
        let mut client = MockDockerClient::new();
        expect_list_containers(&mut client, Ok(vec![container_summary()]));
        expect_inspect_container(&mut client, Ok(Default::default()));
        let client = Client { client };

        let workspace = client
            .get_workspace(&GetWorkspaceParam {
                config: WORKSPACE_FOLDER.join(".devcontainer/devcontainer.json"),
                folder: WORKSPACE_FOLDER.clone(),
            })
            .await?;

        assert_eq!(workspace.containers.len(), 1);
        assert_eq!(workspace.containers[0].names, vec!["devcontainer-app-1"]);
        Ok(())
    }

    #[tokio::test]
    async fn get_workspace_fails_when_docker_errors() -> Result<()> {
        let mut client = MockDockerClient::new();
        expect_list_containers(&mut client, Err(anyhow::anyhow!("oh no")));
        let client = Client { client };

        let result = client
            .get_workspace(&GetWorkspaceParam {
                config: WORKSPACE_FOLDER.join(".devcontainer/devcontainer.json"),
                folder: WORKSPACE_FOLDER.clone(),
            })
            .await;

        assert_eq!(result.unwrap_err().to_string(), "oh no");
        Ok(())
    }

    #[tokio::test]
    async fn workspace_displays_as_text() -> Result<()> {
        let mut client = MockDockerClient::new();
        expect_list_containers(&mut client, Ok(vec![container_summary()]));
        expect_inspect_container(&mut client, Ok(Default::default()));
        let client = Client { client };

        let workspace = client
            .get_workspace(&GetWorkspaceParam {
                config: WORKSPACE_FOLDER.join(".devcontainer/devcontainer.json"),
                folder: WORKSPACE_FOLDER.clone(),
            })
            .await?;

        let expected = indoc::formatdoc! {"
            Workspace: {folder}

            Containers:
              devcontainer-app-1
                Image: mcr.microsoft.com/devcontainers/rust:latest
                Ports: 8080 → 127.0.0.1:8080

            Environment:
              DATABASE_URL = postgres://db:5432/postgres
            ",
            folder = WORKSPACE_FOLDER.display()
        };
        assert_eq!(workspace.to_string(), expected);
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
            }],
            variables: vec![],
        };

        let output = workspace.to_string();
        assert!(output.contains("Containers:"));
        assert!(!output.contains("Environment:"));
    }

    #[test]
    fn spec_resolves_environment() {
        let container_env = HashMap::from([("ALLOWED_VAR".to_string(), "default".to_string())]);

        let spec = Spec {
            remote_env: None,
            container_env: Some(container_env),
        };

        let resolved = spec.resolve_env(&vec!["ALLOWED_VAR=real_value".to_string()]);
        assert_eq!(resolved.get("ALLOWED_VAR").unwrap(), "real_value");
    }

    #[test]
    fn spec_filters_undeclared_variables() {
        let spec = Spec {
            remote_env: None,
            container_env: Some(HashMap::from([(
                "ALLOWED_VAR".to_string(),
                "default".to_string(),
            )])),
        };

        let resolved = spec.resolve_env(&vec![
            "ALLOWED_VAR=allowed".to_string(),
            "IGNORED_VAR=ignored".to_string(),
        ]);

        assert_eq!(resolved.get("ALLOWED_VAR").unwrap(), "allowed");
        assert_eq!(resolved.get("IGNORED_VAR"), None);
    }

    #[test]
    fn spec_uses_default_when_key_absent_from_runtime_env() {
        let spec = Spec {
            remote_env: None,
            container_env: Some(HashMap::from([(
                "MISSING_VAR".to_string(),
                "default_value".to_string(),
            )])),
        };

        let resolved = spec.resolve_env(&vec![]);
        assert_eq!(resolved.get("MISSING_VAR").map(String::as_str), Some("default_value"));
    }

    #[test]
    fn spec_returns_empty_when_container_env_is_none() {
        let spec = Spec {
            remote_env: None,
            container_env: None,
        };

        let resolved = spec.resolve_env(&vec!["SOME_VAR=value".to_string()]);
        assert!(resolved.is_empty());
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
}
