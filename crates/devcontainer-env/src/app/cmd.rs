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
                for entry in workspace.variables {
                    writeln!(self.writer, "export {}={}", entry.key, entry.value)?;
                }
            }
            ExportFormat::Json => {
                let environment: HashMap<String, String> = VariableVec(workspace.variables).into();
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
        let environment: HashMap<String, String> = VariableVec(workspace.variables).into();

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

        fn contents(&self) -> Vec<u8> {
            self.0.lock().unwrap().clone()
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

    fn setup() -> MockWorkspaceClient {
        let mut client = MockWorkspaceClient::new();
        // Mock the get_workspace method.
        client.expect_get_workspace().return_once(|params| {
            let folder = params.folder.clone();
            let config = params.config.clone();

            Box::pin(async move {
                Ok(Workspace {
                    folder,
                    config,
                    containers: vec![Container {
                        names: vec!["devcontainer-app-1".to_string()],
                        image: "mcr.microsoft.com/devcontainers/rust:latest".to_string(),
                        hosts: vec!["my-host".to_string()],
                        ports: vec![PortMapping {
                            host_ip: "127.0.0.1".to_string(),
                            host_port: 8080,
                            container_port: 8080,
                            protocol: "tcp".to_string(),
                        }],
                    }],
                    variables: vec![Variable {
                        key: String::from("FAKE_VAR"),
                        value: String::from("brown-fox"),
                    }],
                })
            })
        });

        client
    }

    #[tokio::test]
    async fn export_environment_for_bash() -> Result<()> {
        let client = setup();
        let writer = Writer::new();
        let reader = writer.clone();

        let mut command = ExportCommand {
            writer: Box::new(writer),
            client: Box::new(client),
        };

        let args = ExportCommandArgs {
            parent: ProgramArgs::default(),
            format: ExportFormat::Bash,
        };

        command.execute(&args).await?;

        let output = String::from_utf8(reader.contents()).unwrap();
        assert_eq!(output, "export FAKE_VAR=brown-fox\n");

        Ok(())
    }

    #[tokio::test]
    async fn export_environment_as_json() -> Result<()> {
        let client = setup();
        let writer = Writer::new();
        let reader = writer.clone();

        let mut command = ExportCommand {
            writer: Box::new(writer),
            client: Box::new(client),
        };

        let args = ExportCommandArgs {
            parent: ProgramArgs::default(),
            format: ExportFormat::Json,
        };

        command.execute(&args).await?;

        let output = String::from_utf8(reader.contents()).unwrap();
        assert_eq!(output, "{\"FAKE_VAR\":\"brown-fox\"}\n");

        Ok(())
    }

    #[tokio::test]
    async fn export_environment_fails() -> Result<()> {
        let mut client = MockWorkspaceClient::new();
        client
            .expect_get_workspace()
            .return_once(|_| Box::pin(async move { Err(anyhow::anyhow!("oh no")) }));
        let writer = Writer::new();

        let mut command = ExportCommand {
            writer: Box::new(writer),
            client: Box::new(client),
        };

        let args = ExportCommandArgs {
            parent: ProgramArgs::default(),
            format: ExportFormat::Json,
        };

        let result = command.execute(&args).await;
        assert_eq!(result.unwrap_err().to_string(), "oh no", "expected error");

        Ok(())
    }

    #[tokio::test]
    async fn inspect_workspace() -> Result<()> {
        let args = InspectCommandArgs {
            parent: ProgramArgs::default(),
        };

        let client = setup();
        let writer = Writer::new();
        let reader = writer.clone();
        // Prepare the command
        let mut command = InspectCommand {
            writer: Box::new(writer),
            client: Box::new(client),
        };

        command.execute(&args).await?;
        let output = String::from_utf8(reader.contents()).unwrap();
        let workspace = indoc::indoc! {"
        Workspace: .

        Containers:
          devcontainer-app-1
            Image: mcr.microsoft.com/devcontainers/rust:latest
            Hosts: my-host
            Ports: 8080 → 127.0.0.1:8080

        Environment:
          FAKE_VAR = brown-fox
        "};
        assert_eq!(output, workspace);

        Ok(())
    }

    #[tokio::test]
    async fn inspect_workspace_fails() -> Result<()> {
        let args = InspectCommandArgs {
            parent: ProgramArgs::default(),
        };

        let mut client = MockWorkspaceClient::new();
        client
            .expect_get_workspace()
            .return_once(|_| Box::pin(async move { Err(anyhow::anyhow!("oh no")) }));
        let writer = Writer::new();
        // Prepare the command
        let mut command = InspectCommand {
            writer: Box::new(writer),
            client: Box::new(client),
        };
        let result = command.execute(&args).await;
        assert_eq!(result.unwrap_err().to_string(), "oh no", "expected error");

        Ok(())
    }

    #[tokio::test]
    async fn exec_command() -> Result<()> {
        let client = setup();

        let mut command = ExecCommand {
            client: Box::new(client),
        };

        let args = ExecCommandArgs {
            parent: ProgramArgs::default(),
            command: vec![
                String::from("sh"),
                String::from("-c"),
                String::from(r#"[ "$FAKE_VAR" = "brown-fox" ]"#),
                String::from("exit 0"),
            ],
        };

        let result = command.execute(&args).await;
        assert!(result.is_ok(), "expected success");

        Ok(())
    }

    #[tokio::test]
    async fn exec_command_fails() -> Result<()> {
        let mut client = MockWorkspaceClient::new();
        client
            .expect_get_workspace()
            .return_once(|_| Box::pin(async move { Err(anyhow::anyhow!("oh no")) }));

        let mut command = ExecCommand {
            client: Box::new(client),
        };

        let args = ExecCommandArgs {
            parent: ProgramArgs::default(),
            command: vec![String::from("sh")],
        };

        let result = command.execute(&args).await;
        assert_eq!(result.unwrap_err().to_string(), "oh no", "expected error");

        Ok(())
    }

    #[tokio::test]
    async fn exec_command_fails_with_status() -> Result<()> {
        let client = setup();

        let mut command = ExecCommand {
            client: Box::new(client),
        };

        let args = ExecCommandArgs {
            parent: ProgramArgs::default(),
            command: vec![
                String::from("sh"),
                String::from("-c"),
                String::from("exit 1"),
            ],
        };

        let result = command.execute(&args).await;
        assert_eq!(
            result.unwrap_err().to_string(),
            "command exited with status exit status: 1",
            "expected error"
        );

        Ok(())
    }
}
