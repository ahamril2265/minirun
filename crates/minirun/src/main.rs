use clap::{Parser, Subcommand};
use std::process::{exit, Command};

/// minirun: a tiny container runtime
#[derive(Parser)]
#[command(name = "minirun", version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run a command inside a new container
    Run {
        /// Memory limit, e.g. 100M (not enforced yet)
        #[arg(long)]
        memory: Option<String>,

        /// Maximum number of processes (not enforced yet)
        #[arg(long)]
        pids: Option<u32>,

        /// Command to run, followed by its arguments
        #[arg(required = true, trailing_var_arg = true, allow_hyphen_values = true)]
        cmd: Vec<String>,
    },
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Run { memory, pids, cmd } => {
            println!("[minirun] memory={memory:?} pids={pids:?} cmd={cmd:?}");
            let code = run(&cmd);
            exit(code);
        }
    }
}

/// Start cmd[0] with the remaining items as arguments, wait, return its exit code.
fn run(cmd: &[String]) -> i32 {
    println!("[minirun] parent pid = {}", std::process::id());

    let status = Command::new(&cmd[0])
        .args(&cmd[1..])
        .status()
        .expect("failed to start command");

    println!("[minirun] child finished: {status}");
    status.code().unwrap_or(1)
}
