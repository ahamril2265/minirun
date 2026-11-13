use clap::{Parser, Subcommand};
use nix::mount::{mount, MsFlags};
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

/// Start the command in new PID + UTS + mount namespaces, with its own /proc.
fn run(cmd: &[String], hostname: String) -> i32 {
    println!("[minirun] parent pid (host) = {}", std::process::id());

    // PID namespace: called in the PARENT, affects our NEXT child.
    unshare(CloneFlags::CLONE_NEWPID)
        .expect("unshare(CLONE_NEWPID) failed - did you use sudo?");

    let mut command = Command::new(&cmd[0]);
    command.args(&cmd[1..]);

    // Runs in the CHILD (already PID 1 of the new PID namespace), before execve.
    unsafe {
        command.pre_exec(move || {
            // 1. New hostname + new mount table
            unshare(CloneFlags::CLONE_NEWUTS | CloneFlags::CLONE_NEWNS)
                .map_err(io::Error::from)?;

            // 2. Hostname
            sethostname(&hostname).map_err(io::Error::from)?;

            // 3. Make every mount private so nothing leaks back to the host
            mount(
                None::<&str>,
                "/",
                None::<&str>,
                MsFlags::MS_REC | MsFlags::MS_PRIVATE,
                None::<&str>,
            )
            .map_err(io::Error::from)?;

            // 4. Fresh /proc that reflects OUR PID namespace
            mount(
                Some("proc"),
                "/proc",
                Some("proc"),
                MsFlags::MS_NOSUID | MsFlags::MS_NODEV | MsFlags::MS_NOEXEC,
                None::<&str>,
            )
            .map_err(io::Error::from)?;

            Ok(())
        });
    }

    let status = command.status().expect("failed to start command");

    println!("[minirun] child finished: {status}");
    status.code().unwrap_or(1)
}
