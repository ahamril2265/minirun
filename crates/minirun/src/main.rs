use clap::{Parser, Subcommand};
use nix::sched::{unshare, CloneFlags};
use nix::unistd::sethostname;
use std::io;
use std::os::unix::process::CommandExt;
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

        /// Hostname inside the container
        #[arg(long, default_value = "minirun")]
        hostname: String,

        /// Command to run, followed by its arguments
        #[arg(required = true, trailing_var_arg = true, allow_hyphen_values = true)]
        cmd: Vec<String>,
    },
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Run { memory, pids, hostname, cmd } => {
            println!("[minirun] memory={memory:?} pids={pids:?} hostname={hostname} cmd={cmd:?}");
            let code = run(&cmd, hostname);
            exit(code);
        }
    }
}

/// Start the command in a new UTS namespace with its own hostname.
fn run(cmd: &[String], hostname: String) -> i32 {
    println!("[minirun] parent pid = {}", std::process::id());

    let mut command = Command::new(&cmd[0]);
    command.args(&cmd[1..]);

    // Runs in the CHILD, after clone3 and before execve.
    unsafe {
        command.pre_exec(move || {
            unshare(CloneFlags::CLONE_NEWUTS).map_err(io::Error::from)?;
            sethostname(&hostname).map_err(io::Error::from)?;
            Ok(())
        });
    }

    let status = command.status().expect("failed to start command");

    println!("[minirun] child finished: {status}");
    status.code().unwrap_or(1)
}
