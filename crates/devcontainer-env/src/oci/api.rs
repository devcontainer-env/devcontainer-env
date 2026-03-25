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
    /// Running containers associated with this workspace.
    pub containers: Vec<Container>,
    /// Environment variables exported by the devcontainer services.
    pub environment: Vec<Variable>,
}

/// A single environment variable as a key-value pair.
#[derive(Debug)]
pub struct Variable {
    /// The variable name.
    pub key: String,
    /// The variable value.
    pub value: String,
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
    /// Container name.
    pub name: String,
    /// Image name used to create the container.
    pub image: String,
    /// Port mappings exposed by this container.
    pub ports: Vec<PortMapping>,
}

/// Describes a single port mapping between a container port and a host port.
#[derive(Debug)]
pub struct PortMapping {
    /// Port number inside the container.
    pub container_port: u16,
    /// Corresponding port number on the host.
    pub host_port: u16,
    /// Transport protocol (e.g. `"tcp"` or `"udp"`).
    pub protocol: String,
}

/// Parameters for [`Client::get_workspace`].
pub struct GetWorkspaceParam {
    /// Path to the devcontainer.json configuration file.
    pub config: PathBuf,
    /// Path to the workspace root folder.
    pub folder: PathBuf,
}

#[async_trait]
pub trait WorkspaceClient {
    async fn get_workspace(&self, args: &GetWorkspaceParam) -> Result<Workspace>;
}

/// Docker client for querying devcontainer workspace state via the OCI/Docker API.
pub struct Client {
    client: Docker,
}

impl Client {
    /// Creates a new [`Client`] connected via the default Docker socket.
    ///
    /// Returns an error if the Docker socket cannot be reached.
    pub fn new() -> Result<Client> {
        let client = Docker::connect_with_socket_defaults()?;
        Ok(Self { client })
    }
}

#[async_trait]
#[cfg_attr(test, automock)]
impl WorkspaceClient for Client {
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
        // TODO: replace with real Docker queries
        let containers = vec![Container {
            name: "devcontainer-app-1".to_string(),
            image: "mcr.microsoft.com/devcontainers/rust:latest".to_string(),
            ports: vec![PortMapping {
                container_port: 8080,
                host_port: 8080,
                protocol: "tcp".to_string(),
            }],
        }];
        let environment = vec![
            Variable {
                key: "RUST_LOG".to_string(),
                value: "debug".to_string(),
            },
            Variable {
                key: "DATABASE_URL".to_string(),
                value: "postgres://localhost:5432/dev".to_string(),
            },
        ];

        Ok(Workspace {
            folder,
            config,
            containers,
            environment,
        })
    }
}
