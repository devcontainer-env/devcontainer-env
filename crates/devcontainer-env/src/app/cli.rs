use clap::{Args, Parser, Subcommand, ValueEnum};
use core::fmt::Display;
use std::path::{Path, PathBuf};

/// Program is the main entry point for the devcontainer-env CLI.
#[derive(Debug, Parser)]
#[command(
    name = "devcontainer-env",
    about = "Bridge devcontainers and the host environment.",
    long_about = "Run host commands with devcontainer service environments and automatically rewrite container service URLs to host ports.",
    version
)]
pub struct Program {
    /// Command specifies the subcommand to execute.
    #[command(subcommand)]
    pub command: ProgramCommand,
}

/// ProgramArgs holds the shared global flags available to every subcommand.
#[derive(Debug, Args)]
pub struct ProgramArgs {
    /// Path to the devcontainer.json configuration file.
    #[arg(
        help = "devcontainer.json path.",
        default_value = ".devcontainer/devcontainer.json",
        long
    )]
    pub config: PathBuf,

    /// Path to the workspace folder containing the devcontainer.
    #[arg(help = "Workspace folder path.", default_value = ".", long)]
    pub workspace_folder: PathBuf,
}

impl Default for ProgramArgs {
    fn default() -> Self {
        Self {
            config: Path::new(".devcontainer/devcontainer.json").into(),
            workspace_folder: Path::new(".").into(),
        }
    }
}

/// Top-level subcommand dispatched by [`Program`].
#[derive(Debug, Subcommand)]
pub enum ProgramCommand {
    /// Inspect and display the devcontainer environment configuration and service port mappings.
    #[command(
        name = "inspect",
        about = "Inspect and display the devcontainer environment configuration and service port mappings.",
        long_about = "Parse the devcontainer.json configuration and print all resolved environment variables and forwarded service port mappings in the requested format."
    )]
    Inspect(InspectCommandArgs),

    /// Export devcontainer service environment variables with container URLs rewritten to host ports.
    #[command(
        name = "export",
        about = "Export devcontainer service environment variables with container URLs rewritten to host ports.",
        long_about = "Resolve the devcontainer service environment and emit shell-ready export statements (or JSON) with every container service URL rewritten to the corresponding forwarded host port."
    )]
    Export(ExportCommandArgs),

    /// Execute a host command with the devcontainer service environment applied, rewriting container URLs to host ports.
    #[command(
        name = "exec",
        about = "Execute a host command with the devcontainer service environment applied, rewriting container URLs to host ports.",
        long_about = "Inject the resolved devcontainer service environment—rewriting container URLs to host ports—into the current process environment, then exec the given command so it inherits that environment."
    )]
    Exec(ExecCommandArgs),
}

/// InspectCommandArgs defines the arguments for the InspectCommand.
#[derive(Debug, Args)]
pub struct InspectCommandArgs {
    /// Shared global flags.
    #[command(flatten)]
    pub parent: ProgramArgs,
}

/// ExportFormat specifies the output format for the export subcommand.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, ValueEnum)]
pub enum ExportFormat {
    /// Bash format.
    #[default]
    Bash,

    /// Json format.
    Json,
}

impl Display for ExportFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Bash => write!(f, "bash"),
            Self::Json => write!(f, "json"),
        }
    }
}

/// ExportCommandArgs defines the arguments for the ExportCommand.
#[derive(Debug, Args)]
pub struct ExportCommandArgs {
    /// Shared global flags.
    #[command(flatten)]
    pub parent: ProgramArgs,

    /// Output format for the exported environment variables.
    #[arg(help = "Output format.", default_value_t = ExportFormat::Bash, long, short)]
    pub format: ExportFormat,
}

/// ExecCommandArgs defines the arguments for the ExecCommand.
#[derive(Debug, Args)]
pub struct ExecCommandArgs {
    /// Shared global flags.
    #[command(flatten)]
    pub parent: ProgramArgs,

    /// Command and arguments to execute.
    #[arg(
        help = "Command and arguments to execute.",
        required = true,
        trailing_var_arg = true
    )]
    pub command: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn export_format_displays_as_bash() {
        let output = ExportFormat::Bash;
        assert_eq!(output.to_string(), "bash")
    }

    #[test]
    fn export_format_displays_as_json() {
        let output = ExportFormat::Json;
        assert_eq!(output.to_string(), "json")
    }

    #[test]
    fn program_args_defaults() {
        let args = ProgramArgs::default();
        assert_eq!(args.workspace_folder, Path::new("."));
        assert_eq!(args.config, Path::new(".devcontainer/devcontainer.json"));
    }
}
