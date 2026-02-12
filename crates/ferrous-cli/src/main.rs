use clap::{Parser, Subcommand};
use ferrous_runtime::Runtime;
use std::error::Error;
use std::path::PathBuf;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run an ELF binary
    Run {
        /// Path to the ELF file
        file: PathBuf,

        /// Memory size in bytes
        #[arg(long, default_value_t = 1024 * 1024)]
        memory: usize,
    },
}

fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Run { file, memory } => {
            println!("Starting Ferrous VM with {} bytes memory...", memory);
            println!("Loading binary: {:?}", file);

            let mut runtime = Runtime::new(memory)?;
            runtime.load_program(&file)?;
            runtime.run()?;

            println!("Execution completed.");
        }
    }

    Ok(())
}
