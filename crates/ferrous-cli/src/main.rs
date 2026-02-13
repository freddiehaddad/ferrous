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
        #[arg(long, default_value_t = 16 * 1024 * 1024)]
        memory: usize,

        /// Path to the disk image
        #[arg(long)]
        disk: Option<PathBuf>,
    },
}

fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Run { file, memory, disk } => {
            println!("Starting Ferrous VM with {} bytes memory...", memory);
            println!("Loading binary: {:?}", file);
            if let Some(d) = &disk {
                println!("Mounting disk image: {:?}", d);
            }

            let mut runtime = Runtime::new(memory, disk.as_deref())?;
            runtime.load_program(&file)?;
            runtime.run()?;

            println!("Execution completed.");
        }
    }

    Ok(())
}
