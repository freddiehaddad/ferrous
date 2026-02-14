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
    RunShell {
        /// Start the UDP echo server in the background
        #[arg(long)]
        with_net: bool,
    },
    /// Run network test (launches UDP echo server + VM)
    RunNet,
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
                "net_test",
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
                "net_test",
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
        Commands::RunShell { with_net } => {
            // Create FS first (which builds user programs)
            run_xtask(&sh, &["fs"])?;

            // Start UDP Echo Server if requested
            let mut server = None;
            if with_net {
                use std::process::{Command, Stdio};

                // Build the tool first
                println!("Building UDP Echo Server...");
                cmd!(sh, "cargo build -p udp-echo --release").run()?;

                let tool_path = if cfg!(windows) {
                    "target/release/udp-echo.exe"
                } else {
                    "target/release/udp-echo"
                };

                println!("Starting UDP Echo Server...");
                let s = Command::new(tool_path)
                    .stdout(Stdio::inherit())
                    .stderr(Stdio::inherit())
                    .spawn()?;
                server = Some(s);
                // Give it a moment to start
                std::thread::sleep(std::time::Duration::from_millis(500));
            }

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

            // Kill server if running
            if let Some(mut s) = server {
                let _ = s.kill();
            }

            if !status.success() {
                return Err(anyhow::anyhow!("VM execution failed"));
            }
        }
        Commands::RunNet => {
            // Build user programs first
            run_xtask(&sh, &["build-user"])?;

            // Start UDP Echo Server in background
            use std::process::{Command, Stdio};

            // Build the tool first
            println!("Building UDP Echo Server...");
            cmd!(sh, "cargo build -p udp-echo --release").run()?;

            let tool_path = if cfg!(windows) {
                "target/release/udp-echo.exe"
            } else {
                "target/release/udp-echo"
            };

            println!("Starting UDP Echo Server...");
            let mut server = Command::new(tool_path)
                .stdout(Stdio::inherit())
                .stderr(Stdio::inherit())
                .spawn()?;

            // Give it a moment to start
            std::thread::sleep(std::time::Duration::from_millis(500));

            // Run VM with net_test
            let binary = out_path.join("net_test");
            println!("Running VM with net_test...");
            let status = Command::new("cargo")
                .args(["run", "-p", "ferrous-cli", "--", "run"])
                .arg(binary)
                // Add network flag if ferrous-cli supports it, or it might be default?
                // Assuming default or transparent
                .status();

            // Kill server
            let _ = server.kill();

            match status {
                Ok(s) if s.success() => println!("VM finished successfully"),
                _ => return Err(anyhow::anyhow!("VM execution failed")),
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
