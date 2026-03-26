#![allow(dead_code)]

use anyhow::Result;
use async_trait::async_trait;
use bollard::Docker;
use std::{collections::HashMap, path::PathBuf};

#[cfg(test)]
use mockall::automock;

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
pub struct VariableVec(pub Vec<Variable>);

impl From<VariableVec> for HashMap<String, String> {
    fn from(vars: VariableVec) -> Self {
        vars.0.into_iter().map(|var| (var.key, var.value)).collect()
    }
}

/// Represents a running Docker container within the devcontainer workspace.
#[derive(Debug)]
pub struct Container {
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

pub struct ContainerFilter {
    pub name: String,
    pub value: String,
}

impl std::fmt::Display for ContainerFilter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}={}", self.name, self.value)
    }
}

pub struct ContainerFilters(pub Vec<ContainerFilter>);

impl From<ContainerFilters> for bollard::query_parameters::ListContainersOptions {
    fn from(filters: ContainerFilters) -> Self {
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

#[async_trait]
#[cfg_attr(test, automock)]
pub trait DockerClient {
    async fn list_containers(
        &self,
        opts: Option<bollard::query_parameters::ListContainersOptions>,
    ) -> Result<Vec<bollard::plugin::ContainerSummary>>;
}

#[async_trait]
impl DockerClient for Docker {
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

#[async_trait]
#[cfg_attr(test, automock)]
pub trait WorkspaceClient {
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
        let config = args.config.canonicalize()?;
        let folder = args.folder.canonicalize()?;

        let variables = Vec::new();
        let mut containers = Vec::new();

        let filters = vec![
            ContainerFilter {
                name: String::from("com.docker.compose.project.working_dir"),
                value: format!("{}", config.parent().unwrap().display()),
            },
            ContainerFilter {
                name: String::from("devcontainer.local_folder"),
                value: format!("{}", folder.display()),
            },
        ];

        for filter in filters {
            // Prepare the options
            let opts = Some(ContainerFilters(vec![filter]).into());
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
            .unwrap()
    });

    fn setup() -> MockDockerClient {
        let mut client = MockDockerClient::new();
        client.expect_list_containers().return_once(|_| {
            Box::pin(async move {
                Ok(vec![bollard::plugin::ContainerSummary {
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
                }])
            })
        });

        client
    }

    #[tokio::test]
    async fn client_returns_workspace() -> Result<()> {
        let args = GetWorkspaceParam {
            config: WORKSPACE_FOLDER.join(".devcontainer/devcontainer.json"),
            folder: WORKSPACE_FOLDER.clone(),
        };

        let client = setup();
        let client = Client { client };
        let workspace = client.get_workspace(&args).await?;
        assert_eq!(workspace.config, args.config);
        assert_eq!(workspace.folder, args.folder);
        assert_eq!(workspace.containers.len(), 1);

        let container = workspace.containers.first().unwrap();
        assert_eq!(container.names, vec!["devcontainer-app-1".to_string()]);
        assert_eq!(container.ports.len(), 1);

        let port = container.ports.first().unwrap();
        assert_eq!(port.container_port, 8080);
        assert_eq!(port.host_ip, "127.0.0.1");
        assert_eq!(port.host_port, 8080);
        assert_eq!(port.protocol, "tcp");

        Ok(())
    }

    #[tokio::test]
    async fn client_returns_workspace_fails() -> Result<()> {
        let args = GetWorkspaceParam {
            config: WORKSPACE_FOLDER.join(".devcontainer/devcontainer.json"),
            folder: WORKSPACE_FOLDER.clone(),
        };

        let mut client = MockDockerClient::new();
        client
            .expect_list_containers()
            .return_once(|_| Box::pin(async move { anyhow::bail!("oh no") }));

        let client = Client { client };
        let result = client.get_workspace(&args).await;
        assert_eq!(result.unwrap_err().to_string(), "oh no", "expected error");

        Ok(())
    }

    #[tokio::test]
    async fn workspace_displays_as_text() -> Result<()> {
        let args = GetWorkspaceParam {
            config: WORKSPACE_FOLDER.join(".devcontainer/devcontainer.json"),
            folder: WORKSPACE_FOLDER.clone(),
        };

        let client = setup();
        let client = Client { client };
        let workspace = client.get_workspace(&args).await?;
        let workspace_as_string = indoc::formatdoc! {"
            Workspace: {folder}

            Containers:
              devcontainer-app-1
                Image: mcr.microsoft.com/devcontainers/rust:latest
                Ports: 8080 → 127.0.0.1:8080
            ",
            folder = WORKSPACE_FOLDER.display()
        };
        assert_eq!(format!("{}", workspace), workspace_as_string);

        Ok(())
    }
}
