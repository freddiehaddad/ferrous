use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::{Path, PathBuf};
use xshell::{cmd, Shell};

#[derive(Parser)]
#[command(name = "xtask")]
#[command(about = "Ferrous OS build system", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Build host tools (VM, CLI, mkfs)
    BuildHost,
    /// Build user programs (shell, examples) for RISC-V
    BuildUser,
    /// Run simple hello-world example
    RunHello,
    /// Create disk image (disk.img) with shell and examples
    Fs,
    /// Run the interactive shell (requires disk image)
    RunShell,
    /// Clean build artifacts
    Clean,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let sh = Shell::new()?;

    // Ensure we are in the project root
    let project_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    sh.change_dir(project_root);

    let target = "riscv32i-unknown-none-elf";
    let mode = "release";
    let out_dir = format!("target/{}/{}", target, mode);
    let out_path = Path::new(&out_dir);

    match cli.command {
        Commands::BuildHost => {
            cmd!(sh, "cargo build -p ferrous-cli -p ferrous-mkfs").run()?;
        }
        Commands::BuildUser => {
            let flags = vec!["--release", "--target", target];
            let packages = vec![
                "hello-world",
                "shell",
                "echo",
                "threads",
                "sbrk",
                "file-read",
                "disk-read",
            ];

            // Construct the cargo build command
            // We use .args() to pass dynamic lists of arguments
            let mut cmd = cmd!(sh, "cargo build");
            for flag in &flags {
                cmd = cmd.arg(flag);
            }
            for pkg in packages {
                cmd = cmd.arg("-p").arg(pkg);
            }
            cmd.run()?;
        }
        Commands::RunHello => {
            // Build user programs first
            run_xtask(&sh, &["build-user"])?;

            let binary = out_path.join("hello-world");
            cmd!(sh, "cargo run -p ferrous-cli -- run {binary}").run()?;
        }
        Commands::Fs => {
            // Build user programs first
            run_xtask(&sh, &["build-user"])?;

            println!("Creating hello.txt...");
            sh.write_file("hello.txt", "Hello from Ferrous File System!\n")?;

            println!("Building disk image...");
            let binaries = vec![
                "shell",
                "echo",
                "threads",
                "sbrk",
                "hello-world",
                "file-read",
                "disk-read",
            ];

            let mut paths = Vec::new();
            for bin in binaries {
                paths.push(out_path.join(bin));
            }
            paths.push(PathBuf::from("hello.txt"));

            let mut cmd = cmd!(
                sh,
                "cargo run -p ferrous-mkfs -- --disk disk.img --force --inodes 128"
            );
            for path in paths {
                cmd = cmd.arg(path);
            }
            cmd.run()?;

            sh.remove_path("hello.txt")?;
        }
        Commands::RunShell => {
            // Create FS first (which builds user programs)
            run_xtask(&sh, &["fs"])?;

            let shell_bin = out_path.join("shell");

            // Use std::process::Command directly to ensure stdin is properly inherited
            // for the interactive shell session. xshell can sometimes cause issues
            // with interactive input on some platforms.
            let status = std::process::Command::new("cargo")
                .args(["run", "-p", "ferrous-cli", "--", "run"])
                .arg(shell_bin)
                .arg("--disk")
                .arg("disk.img")
                .stdin(std::process::Stdio::inherit())
                .stdout(std::process::Stdio::inherit())
                .stderr(std::process::Stdio::inherit())
                .status()?;

            if !status.success() {
                return Err(anyhow::anyhow!("VM execution failed"));
            }
        }
        Commands::Clean => {
            cmd!(sh, "cargo clean").run()?;
            if sh.path_exists("disk.img") {
                sh.remove_path("disk.img")?;
            }
        }
    }

    Ok(())
}

// Helper to run recursive xtask commands
fn run_xtask(sh: &Shell, args: &[&str]) -> Result<()> {
    // We can just call the binary recursively, or refactor to call functions.
    // Calling binary is simpler for ensuring clean environment.
    cmd!(sh, "cargo xtask").args(args).run()?;
    Ok(())
}
