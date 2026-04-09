use clap::{Parser, Subcommand};
#[cfg(unix)]
use std::os::unix::process::ExitStatusExt;
use std::path::PathBuf;
use std::process::{self, Command, ExitStatus};

/// DotenvPP CLI — next-generation environment configuration.
#[derive(Parser)]
#[command(name = "dotenvpp", version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Validate that a .env file is parseable and well-formed.
    Check {
        /// Path to the .env file to check.
        #[arg(short, long, default_value = ".env")]
        file: PathBuf,
    },

    /// Load .env variables and run a command.
    Run {
        /// Path to the .env file to load.
        #[arg(short, long, default_value = ".env")]
        file: PathBuf,

        /// The command and its arguments to run.
        #[arg(trailing_var_arg = true, required = true)]
        command: Vec<String>,
    },
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Check {
            file,
        } => match dotenvpp::from_path_iter(&file) {
            Ok(pairs) => {
                let count = pairs.count();
                println!(
                    "✅ {} — {count} variable{} parsed successfully",
                    file.display(),
                    if count == 1 {
                        ""
                    } else {
                        "s"
                    }
                );
            }
            Err(err) => {
                eprintln!("❌ {}: {err}", file.display());
                process::exit(1);
            }
        },
        Commands::Run {
            file,
            command,
        } => {
            // Load env vars (do not override existing).
            if let Err(err) = dotenvpp::from_path(&file) {
                eprintln!("❌ Failed to load {}: {err}", file.display());
                process::exit(1);
            }

            let program = &command[0];
            let args = &command[1..];

            let status = Command::new(program).args(args).status().unwrap_or_else(|err| {
                eprintln!("❌ Failed to execute {program}: {err}");
                process::exit(1);
            });

            exit_from_status(status);
        }
    }
}

fn exit_from_status(status: ExitStatus) -> ! {
    if let Some(code) = status.code() {
        process::exit(code);
    }

    #[cfg(unix)]
    if let Some(signal) = status.signal() {
        process::exit(128 + signal);
    }

    process::exit(1);
}
