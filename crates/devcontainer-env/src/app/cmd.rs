use crate::app::cli::*;
use crate::oci::api::*;
use anyhow::Result;
use std::collections::HashMap;
use std::io::Write;

/// Export devcontainer service environment variables with container URLs rewritten to host ports.
pub struct ExportCommand {
    pub client: Box<dyn WorkspaceClient + Send + Sync>,
    pub writer: Box<dyn Write>,
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
                for entry in workspace.environment {
                    writeln!(self.writer, "export {}={}", entry.key, entry.value)?;
                }
            }
            ExportFormat::Json => {
                let environment: HashMap<String, String> =
                    VariableVec(workspace.environment).into();
                writeln!(self.writer, "{}", serde_json::to_string(&environment)?)?;
            }
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

    fn setup() -> MockClient {
        let mut client = MockClient::new();
        // Mock the get_workspace method.
        client.expect_get_workspace().return_once(|params| {
            let folder = params.folder.clone();
            let config = params.config.clone();

            Box::pin(async move {
                Ok(Workspace {
                    folder,
                    config,
                    containers: vec![],
                    environment: vec![Variable {
                        key: String::from("FOO_VAR"),
                        value: String::from("brown-fox"),
                    }],
                })
            })
        });

        client
    }

    #[tokio::test]
    async fn exports_environment_for_bash() -> Result<()> {
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
        assert_eq!(output, "export FOO_VAR=brown-fox\n");

        Ok(())
    }

    #[tokio::test]
    async fn exports_environment_as_json() -> Result<()> {
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
        assert_eq!(output, "{\"FOO_VAR\":\"brown-fox\"}\n");

        Ok(())
    }

    #[tokio::test]
    async fn export_environment_fails() -> Result<()> {
        let mut client = MockClient::new();
        client
            .expect_get_workspace()
            .return_once(|_| Box::pin(async move { Err(anyhow::anyhow!("oh no")) }));

        let buffer = Vec::new();
        let mut command = ExportCommand {
            writer: Box::new(buffer),
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
}
