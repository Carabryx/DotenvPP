use clap::{Args, Parser, Subcommand};
#[cfg(unix)]
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::{self, Command, ExitStatus};

/// DotenvPP CLI — next-generation environment configuration.
#[derive(Debug, Parser)]
#[command(name = "dotenvpp", version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Args, Clone, Debug, Default)]
struct SourceArgs {
    /// Path to a specific .env file to load or check.
    #[arg(short, long, value_name = "FILE", conflicts_with = "environment")]
    file: Option<PathBuf>,

    /// Environment name for layered loading, such as `development` or `production`.
    #[arg(short = 'e', long = "env", value_name = "ENV", conflicts_with = "file")]
    environment: Option<String>,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Validate a specific file or the layered environment stack.
    Check {
        #[command(flatten)]
        source: SourceArgs,
    },

    /// Load .env variables and run a command.
    Run {
        #[command(flatten)]
        source: SourceArgs,

        /// The command and its arguments to run.
        #[arg(trailing_var_arg = true, required = true)]
        command: Vec<String>,
    },
}

#[derive(Debug)]
enum RunError {
    MissingCommand,
    Load(dotenvpp::Error),
    Execute {
        program: String,
        source: std::io::Error,
    },
}

fn check_target(file: Option<&Path>, environment: Option<&str>) -> dotenvpp::Result<usize> {
    match file {
        Some(path) => dotenvpp::from_path_iter(path).map(|pairs| pairs.count()),
        None => dotenvpp::from_layered_env(environment).map(|pairs| pairs.len()),
    }
}

fn load_and_run(
    file: Option<&Path>,
    environment: Option<&str>,
    command: &[String],
) -> Result<ExitStatus, RunError> {
    if command.is_empty() {
        return Err(RunError::MissingCommand);
    }

    match file {
        Some(path) => {
            dotenvpp::from_path(path).map_err(RunError::Load)?;
        }
        None => {
            if let Some(environment) = environment {
                dotenvpp::load_with_env(environment).map_err(RunError::Load)?;
            } else {
                dotenvpp::load().map_err(RunError::Load)?;
            }
        }
    }

    let program = &command[0];
    let args = &command[1..];
    let status = Command::new(program).args(args).status().map_err(|source| RunError::Execute {
        program: program.clone(),
        source,
    })?;

    Ok(status)
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Check {
            source,
        } => match check_target(source.file.as_deref(), source.environment.as_deref()) {
            Ok(count) => {
                let target = describe_target(source.file.as_deref(), source.environment.as_deref());
                println!(
                    "✅ {target} — {count} variable{} parsed successfully",
                    if count == 1 {
                        ""
                    } else {
                        "s"
                    }
                );
            }
            Err(err) => {
                let target = describe_target(source.file.as_deref(), source.environment.as_deref());
                eprintln!("❌ {target}: {err}");
                process::exit(1);
            }
        },
        Commands::Run {
            source,
            command,
        } => match load_and_run(source.file.as_deref(), source.environment.as_deref(), &command) {
            Ok(status) => exit_from_status(status),
            Err(RunError::MissingCommand) => {
                eprintln!("❌ No command specified");
                process::exit(1);
            }
            Err(RunError::Load(err)) => {
                let target = describe_target(source.file.as_deref(), source.environment.as_deref());
                eprintln!("❌ Failed to load {target}: {err}");
                process::exit(1);
            }
            Err(RunError::Execute {
                program,
                source,
            }) => {
                eprintln!("❌ Failed to execute {program}: {source}");
                process::exit(1);
            }
        },
    }
}

fn describe_target(file: Option<&Path>, environment: Option<&str>) -> String {
    match file {
        Some(path) => path.display().to_string(),
        None => match environment {
            Some(environment) => format!("layered environment for `{environment}`"),
            None => "layered environment".to_owned(),
        },
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
