use std::env;
use std::process::{exit, Command};

fn main() {
    // Collect command-line arguments: [program_name, "run", command, args...]
    let args: Vec<String> = env::args().collect();

    if args.len() < 3 || args[1] != "run" {
        eprintln!("usage: minirun run <command> [args...]");
        exit(1);
    }

    let program = &args[2];
    let program_args = &args[3..];

    println!("[minirun] parent pid = {}", std::process::id());

    // clone + execve + wait4, all inside .status()
    let status = Command::new(program)
        .args(program_args)
        .status()
        .expect("failed to start command");

    println!("[minirun] child finished: {status}");

    // Pass the child's exit code through as our own
    exit(status.code().unwrap_or(1));
}
