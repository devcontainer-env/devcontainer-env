use crate::app::cli::*;
use crate::oci::api::*;
use anyhow::Result;
use std::collections::{HashMap, VecDeque};
use std::io::Write;
use std::process::{Command, Stdio};

/// Export devcontainer service environment variables with container URLs rewritten to host ports.
pub struct ExportCommand {
    /// Writer used to output the exported environment variables.
    pub writer: Box<dyn Write>,
    /// Client used to retrieve the workspace and its environment.
    pub client: Box<dyn WorkspaceClient + Send + Sync>,
}

impl ExportCommand {
    /// Execute the ExportCommand with the provided arguments.
    pub async fn execute(&mut self, args: &ExportCommandArgs) -> Result<()> {
        // Get the workspace
        let params = &GetWorkspaceParam {
            config: args.parent.config.clone(),
            folder: args.parent.workspace_folder.clone(),
        };
        let workspace = self.client.get_workspace(params).await?;

        // Export the workspace
        match args.format {
            ExportFormat::Bash => {
                for entry in workspace.environment.variables {
                    writeln!(self.writer, "export {}={}", entry.key, entry.value)?;
                }
            }
            ExportFormat::Json => {
                let environment: HashMap<String, String> = HashMap::from(workspace.environment);
                writeln!(self.writer, "{}", serde_json::to_string(&environment)?)?;
            }
        }

        Ok(())
    }
}
pub struct ExecCommand {
    /// Client used to retrieve the workspace and its environment.
    pub client: Box<dyn WorkspaceClient + Send + Sync>,
}

impl ExecCommand {
    /// Execute the ExecCommand with the provided arguments.
    pub async fn execute(&mut self, args: &ExecCommandArgs) -> Result<()> {
        // Get the workspace
        let params = &GetWorkspaceParam {
            config: args.parent.config.clone(),
            folder: args.parent.workspace_folder.clone(),
        };
        let workspace = self.client.get_workspace(params).await?;
        let mut arguments = VecDeque::from(args.command.clone());
        let environment: HashMap<String, String> = HashMap::from(workspace.environment);

        // Prepare the command
        let name = match arguments.pop_front() {
            Some(value) => value,
            None => String::from("sh"),
        };

        // Execute the command
        let status = Command::new(name)
            .args(arguments)
            .envs(&environment)
            .stdout(Stdio::inherit())
            .stdin(Stdio::inherit())
            .stderr(Stdio::inherit())
            .spawn()?
            .wait()?;

        if status.success() {
            Ok(())
        } else {
            anyhow::bail!("command exited with status {}", status);
        }
    }
}

pub struct InspectCommand {
    /// Writer used to output the exported environment variables.
    pub writer: Box<dyn Write>,
    /// Client used to retrieve the workspace and its environment.
    pub client: Box<dyn WorkspaceClient + Send + Sync>,
}

impl InspectCommand {
    /// Execute the ExecCommand with the provided arguments.
    pub async fn execute(&mut self, args: &InspectCommandArgs) -> Result<()> {
        // Get the workspace
        let params = &GetWorkspaceParam {
            config: args.parent.config.clone(),
            folder: args.parent.workspace_folder.clone(),
        };
        let workspace = self.client.get_workspace(params).await?;

        if workspace.containers.is_empty() {
            writeln!(
                self.writer,
                "No running devcontainers found for {}",
                workspace.folder.display()
            )?;
        } else {
            write!(self.writer, "{}", workspace)?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::*;

    #[derive(Clone)]
    struct Writer(Arc<Mutex<Vec<u8>>>);

    impl Writer {
        fn new() -> Self {
            Self(Arc::new(Mutex::new(Vec::new())))
        }

        fn contents(&self) -> String {
            String::from_utf8(self.0.lock().unwrap().clone()).unwrap()
        }
    }

    impl Write for Writer {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(buf);
            Ok(buf.len())
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    fn workspace() -> Workspace {
        Workspace {
            folder: ".".into(),
            config: ".devcontainer/devcontainer.json".into(),
            containers: vec![Container {
                id: "001".to_string(),
                names: vec!["devcontainer-app-1".to_string()],
                image: "mcr.microsoft.com/devcontainers/rust:latest".to_string(),
                hosts: vec!["my-host".to_string()],
                ports: vec![PortMapping {
                    host_ip: "127.0.0.1".to_string(),
                    host_port: 8080,
                    container_port: 8080,
                    protocol: "tcp".to_string(),
                }],
                environment: vec![],
            }],
            environment: Environment {
                variables: vec![Variable {
                    key: String::from("FAKE_VAR"),
                    value: String::from("brown-fox"),
                }],
            },
        }
    }

    fn expect_get_workspace(client: &mut MockWorkspaceClient, res: Result<Workspace>) {
        client
            .expect_get_workspace()
            .return_once(|_| Box::pin(async move { res }));
    }

    #[tokio::test]
    async fn export_writes_bash_export_statements() -> Result<()> {
        let writer = Writer::new();
        let mut client = MockWorkspaceClient::new();
        expect_get_workspace(&mut client, Ok(workspace()));
        let mut cmd = ExportCommand {
            writer: Box::new(writer.clone()),
            client: Box::new(client),
        };

        cmd.execute(&ExportCommandArgs {
            parent: ProgramArgs::default(),
            format: ExportFormat::Bash,
        })
        .await?;

        assert_eq!(writer.contents(), "export FAKE_VAR=brown-fox\n");
        Ok(())
    }

    #[tokio::test]
    async fn export_writes_json_object() -> Result<()> {
        let writer = Writer::new();
        let mut client = MockWorkspaceClient::new();
        expect_get_workspace(&mut client, Ok(workspace()));
        let mut cmd = ExportCommand {
            writer: Box::new(writer.clone()),
            client: Box::new(client),
        };

        cmd.execute(&ExportCommandArgs {
            parent: ProgramArgs::default(),
            format: ExportFormat::Json,
        })
        .await?;

        assert_eq!(writer.contents(), "{\"FAKE_VAR\":\"brown-fox\"}\n");
        Ok(())
    }

    #[tokio::test]
    async fn export_fails_when_client_errors() -> Result<()> {
        let mut client = MockWorkspaceClient::new();
        expect_get_workspace(&mut client, Err(anyhow::anyhow!("oh no")));
        let mut cmd = ExportCommand {
            writer: Box::new(Writer::new()),
            client: Box::new(client),
        };

        let result = cmd
            .execute(&ExportCommandArgs {
                parent: ProgramArgs::default(),
                format: ExportFormat::Json,
            })
            .await;

        assert_eq!(result.unwrap_err().to_string(), "oh no");
        Ok(())
    }

    #[tokio::test]
    async fn inspect_writes_workspace_display() -> Result<()> {
        let writer = Writer::new();
        let mut client = MockWorkspaceClient::new();
        expect_get_workspace(&mut client, Ok(workspace()));
        let mut cmd = InspectCommand {
            writer: Box::new(writer.clone()),
            client: Box::new(client),
        };

        cmd.execute(&InspectCommandArgs {
            parent: ProgramArgs::default(),
        })
        .await?;

        let expected = indoc::indoc! {"
        Workspace: .

        Containers:
          devcontainer-app-1
            Image: mcr.microsoft.com/devcontainers/rust:latest
            Hosts: my-host
            Ports: 8080 → 127.0.0.1:8080

        Environment:
          FAKE_VAR = brown-fox
        "};
        assert_eq!(writer.contents(), expected);
        Ok(())
    }

    #[tokio::test]
    async fn inspect_writes_no_containers_message() -> Result<()> {
        let writer = Writer::new();
        let mut client = MockWorkspaceClient::new();
        expect_get_workspace(
            &mut client,
            Ok(Workspace {
                folder: ".".into(),
                config: ".devcontainer/devcontainer.json".into(),
                containers: vec![],
                environment: Environment { variables: vec![] },
            }),
        );
        let mut cmd = InspectCommand {
            writer: Box::new(writer.clone()),
            client: Box::new(client),
        };

        cmd.execute(&InspectCommandArgs {
            parent: ProgramArgs::default(),
        })
        .await?;

        assert_eq!(writer.contents(), "No running devcontainers found for .\n");
        Ok(())
    }

    #[tokio::test]
    async fn inspect_fails_when_client_errors() -> Result<()> {
        let mut client = MockWorkspaceClient::new();
        expect_get_workspace(&mut client, Err(anyhow::anyhow!("oh no")));
        let mut cmd = InspectCommand {
            writer: Box::new(Writer::new()),
            client: Box::new(client),
        };

        let result = cmd
            .execute(&InspectCommandArgs {
                parent: ProgramArgs::default(),
            })
            .await;

        assert_eq!(result.unwrap_err().to_string(), "oh no");
        Ok(())
    }

    #[tokio::test]
    async fn exec_runs_command_with_env() -> Result<()> {
        let mut client = MockWorkspaceClient::new();
        expect_get_workspace(&mut client, Ok(workspace()));
        let mut cmd = ExecCommand {
            client: Box::new(client),
        };

        cmd.execute(&ExecCommandArgs {
            parent: ProgramArgs::default(),
            command: vec![
                "sh".into(),
                "-c".into(),
                r#"[ "$FAKE_VAR" = "brown-fox" ]"#.into(),
            ],
        })
        .await?;

        Ok(())
    }

    #[tokio::test]
    async fn exec_fails_when_client_errors() -> Result<()> {
        let mut client = MockWorkspaceClient::new();
        expect_get_workspace(&mut client, Err(anyhow::anyhow!("oh no")));
        let mut cmd = ExecCommand {
            client: Box::new(client),
        };

        let result = cmd
            .execute(&ExecCommandArgs {
                parent: ProgramArgs::default(),
                command: vec!["sh".into()],
            })
            .await;

        assert_eq!(result.unwrap_err().to_string(), "oh no");
        Ok(())
    }

    #[tokio::test]
    async fn exec_fails_when_command_exits_nonzero() -> Result<()> {
        let mut client = MockWorkspaceClient::new();
        expect_get_workspace(&mut client, Ok(workspace()));
        let mut cmd = ExecCommand {
            client: Box::new(client),
        };

        let result = cmd
            .execute(&ExecCommandArgs {
                parent: ProgramArgs::default(),
                command: vec!["sh".into(), "-c".into(), "exit 1".into()],
            })
            .await;

        assert_eq!(
            result.unwrap_err().to_string(),
            "command exited with status exit status: 1"
        );
        Ok(())
    }
}
