mod app;
mod oci;

use std::error::Error;

use crate::app::cli::*;
use crate::app::cmd::*;
use crate::oci::api::*;

use clap::Parser;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let client = Box::new(Client::new()?);

    let program = Program::parse();
    // Process the correct command
    match program.command {
        ProgramCommand::Exec(args) => {
            let mut command = ExecCommand { client };
            command.execute(&args).await?
        }
        ProgramCommand::Inspect(args) => {
            let writer = Box::new(std::io::stdout());
            let mut command = InspectCommand { client, writer };
            command.execute(&args).await?
        }
        ProgramCommand::Export(args) => {
            let writer = Box::new(std::io::stdout());
            let mut command = ExportCommand { client, writer };
            command.execute(&args).await?
        }
    }

    Ok(())
}
