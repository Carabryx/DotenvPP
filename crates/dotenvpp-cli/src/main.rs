use clap::{Parser, Subcommand};
#[cfg(unix)]
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
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

#[derive(Debug)]
enum RunError {
    MissingCommand,
    Load(dotenvpp::Error),
    Execute {
        program: String,
        source: std::io::Error,
    },
}

fn check_file(file: &Path) -> dotenvpp::Result<usize> {
    dotenvpp::from_path_iter(file).map(|pairs| pairs.count())
}

fn load_and_run(file: &Path, command: &[String]) -> Result<ExitStatus, RunError> {
    if command.is_empty() {
        return Err(RunError::MissingCommand);
    }

    dotenvpp::from_path(file).map_err(RunError::Load)?;

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
            file,
        } => match check_file(&file) {
            Ok(count) => {
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
        } => match load_and_run(&file, &command) {
            Ok(status) => exit_from_status(status),
            Err(RunError::MissingCommand) => {
                eprintln!("❌ No command specified");
                process::exit(1);
            }
            Err(RunError::Load(err)) => {
                eprintln!("❌ Failed to load {}: {err}", file.display());
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

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;
    use std::fs;
    use std::sync::{Mutex, OnceLock};
    use std::time::{SystemTime, UNIX_EPOCH};

    fn test_lock() -> std::sync::MutexGuard<'static, ()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(())).lock().unwrap_or_else(|e| e.into_inner())
    }

    struct TempEnvFile {
        path: PathBuf,
    }

    impl TempEnvFile {
        fn new(contents: &str) -> Self {
            let mut path = std::env::temp_dir();
            let nanos = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
            path.push(format!("dotenvpp-cli-test-{}-{nanos}.env", std::process::id()));
            fs::write(&path, contents).unwrap();
            Self {
                path,
            }
        }
    }

    impl Drop for TempEnvFile {
        fn drop(&mut self) {
            let _ = fs::remove_file(&self.path);
        }
    }

    #[cfg(windows)]
    fn env_probe_command(key: &str, expected: &str) -> Vec<String> {
        vec![
            "cmd".into(),
            "/C".into(),
            format!("if \"%{key}%\"==\"{expected}\" (exit 0) else (exit 3)"),
        ]
    }

    #[cfg(not(windows))]
    fn env_probe_command(key: &str, expected: &str) -> Vec<String> {
        vec!["sh".into(), "-c".into(), format!("[ \"${key}\" = \"{expected}\" ]")]
    }

    #[test]
    fn check_file_counts_pairs() {
        let _guard = test_lock();
        let file = TempEnvFile::new("A=1\nB=2\n");
        assert_eq!(check_file(&file.path).unwrap(), 2);
    }

    #[test]
    fn check_file_returns_parse_error() {
        let _guard = test_lock();
        let file = TempEnvFile::new("NOT_VALID\n");
        assert!(check_file(&file.path).is_err());
    }

    #[test]
    fn load_and_run_rejects_missing_command() {
        let _guard = test_lock();
        let file = TempEnvFile::new("A=1\n");
        let err = load_and_run(&file.path, &[]).unwrap_err();
        assert!(matches!(err, RunError::MissingCommand));
    }

    #[test]
    fn load_and_run_returns_load_error() {
        let _guard = test_lock();
        let err = load_and_run(Path::new("definitely-missing.env"), &["cmd".into()]).unwrap_err();
        assert!(matches!(err, RunError::Load(_)));
    }

    #[test]
    fn load_and_run_injects_env_into_child_process() {
        let _guard = test_lock();
        let key = "DOTENVPP_CLI_TEST_VALUE";
        let expected = "from_file";
        let file = TempEnvFile::new(&format!("{key}={expected}\n"));
        let command = env_probe_command(key, expected);

        // SAFETY: test cleanup for an isolated process env key.
        unsafe { std::env::remove_var(key) };

        let status = load_and_run(&file.path, &command).unwrap();
        assert!(status.success());

        // SAFETY: test cleanup for an isolated process env key.
        unsafe { std::env::remove_var(key) };
    }

    #[test]
    fn load_and_run_returns_execute_error() {
        let _guard = test_lock();
        let key = "DOTENVPP_CLI_EXEC_ERR";
        let file = TempEnvFile::new(&format!("{key}=1\n"));
        let err = load_and_run(&file.path, &["definitely-not-a-real-command".into()]).unwrap_err();

        assert!(matches!(
            err,
            RunError::Execute {
                program,
                ..
            } if program == "definitely-not-a-real-command"
        ));

        // SAFETY: test cleanup for an isolated process env key.
        unsafe { std::env::remove_var(key) };
    }
}
