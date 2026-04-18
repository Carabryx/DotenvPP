use clap::{Args, Parser, Subcommand};
#[cfg(unix)]
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::{self, Command, ExitStatus};
use std::{env, fs};

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

        /// Schema file to validate against. Defaults to .env.schema when present.
        #[arg(long, value_name = "FILE")]
        schema: Option<PathBuf>,

        /// Enforce policy rules. Defaults to .env.policy when present, otherwise standard rules.
        #[arg(long)]
        strict: bool,

        /// Policy file to evaluate when --strict is set.
        #[arg(long, value_name = "FILE")]
        policy: Option<PathBuf>,
    },

    /// Load .env variables and run a command.
    Run {
        #[command(flatten)]
        source: SourceArgs,

        /// Treat --file as a DotenvPP encrypted .env JSON file.
        #[arg(long)]
        encrypted: bool,

        /// Private key for encrypted files. Defaults to DOTENV_PRIVATE_KEY.
        #[arg(long, value_name = "KEY")]
        private_key: Option<String>,

        /// The command and its arguments to run.
        #[arg(trailing_var_arg = true, required = true)]
        command: Vec<String>,
    },

    /// Schema generation and export tools.
    Schema {
        #[command(subcommand)]
        command: SchemaCommand,
    },

    /// Evaluate policy rules and report all violations.
    Lint {
        #[command(flatten)]
        source: SourceArgs,

        /// Policy file to evaluate. Defaults to .env.policy when present, otherwise standard rules.
        #[arg(long, value_name = "FILE")]
        policy: Option<PathBuf>,
    },

    /// Generate a new X25519 keypair for .env encryption.
    Keygen,

    /// Encrypt a .env file to DotenvPP encrypted JSON.
    Encrypt {
        /// Plaintext .env file.
        #[arg(short, long, value_name = "FILE", default_value = ".env")]
        file: PathBuf,

        /// Recipient public key. Repeat for multiple recipients.
        #[arg(short, long = "recipient", value_name = "PUBLIC_KEY", required = true)]
        recipients: Vec<String>,

        /// Output file. If omitted, encrypted JSON is printed to stdout.
        #[arg(short, long, value_name = "FILE")]
        output: Option<PathBuf>,
    },

    /// Decrypt a DotenvPP encrypted JSON file to stdout.
    Decrypt {
        /// Encrypted file.
        #[arg(short, long, value_name = "FILE", default_value = ".env.enc")]
        file: PathBuf,

        /// Private key. Defaults to DOTENV_PRIVATE_KEY.
        #[arg(long, value_name = "KEY")]
        private_key: Option<String>,
    },

    /// Re-encrypt an encrypted file for a new recipient set.
    Rotate {
        /// Existing encrypted file.
        #[arg(short, long, value_name = "FILE", default_value = ".env.enc")]
        file: PathBuf,

        /// Private key that can decrypt the existing file. Defaults to DOTENV_PRIVATE_KEY.
        #[arg(long, value_name = "KEY")]
        private_key: Option<String>,

        /// New recipient public key. Repeat for multiple recipients.
        #[arg(short, long = "recipient", value_name = "PUBLIC_KEY", required = true)]
        recipients: Vec<String>,

        /// Output file. If omitted, encrypted JSON is printed to stdout.
        #[arg(short, long, value_name = "FILE")]
        output: Option<PathBuf>,
    },
}

#[derive(Debug, Subcommand)]
enum SchemaCommand {
    /// Generate .env.schema TOML from an existing env file.
    Init {
        /// Env file to inspect.
        #[arg(short, long, value_name = "FILE", default_value = ".env")]
        file: PathBuf,

        /// Output file. If omitted, schema TOML is printed to stdout.
        #[arg(short, long, value_name = "FILE")]
        output: Option<PathBuf>,
    },

    /// Generate .env.example from a schema.
    Example {
        /// Schema file.
        #[arg(short, long, value_name = "FILE", default_value = ".env.schema")]
        schema: PathBuf,

        /// Output file. If omitted, example text is printed to stdout.
        #[arg(short, long, value_name = "FILE")]
        output: Option<PathBuf>,
    },

    /// Generate Markdown documentation from a schema.
    Docs {
        /// Schema file.
        #[arg(short, long, value_name = "FILE", default_value = ".env.schema")]
        schema: PathBuf,

        /// Output file. If omitted, docs are printed to stdout.
        #[arg(short, long, value_name = "FILE")]
        output: Option<PathBuf>,
    },

    /// Export JSON Schema from a DotenvPP schema.
    JsonSchema {
        /// Schema file.
        #[arg(short, long, value_name = "FILE", default_value = ".env.schema")]
        schema: PathBuf,

        /// Output file. If omitted, JSON Schema is printed to stdout.
        #[arg(short, long, value_name = "FILE")]
        output: Option<PathBuf>,
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

fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Check {
            source,
            schema,
            strict,
            policy,
        } => check_command(source, schema, strict, policy),
        Commands::Run {
            source,
            encrypted,
            private_key,
            command,
        } => run_command(source, encrypted, private_key, command),
        Commands::Schema {
            command,
        } => schema_command(command),
        Commands::Lint {
            source,
            policy,
        } => lint_command(source, policy),
        Commands::Keygen => keygen_command(),
        Commands::Encrypt {
            file,
            recipients,
            output,
        } => encrypt_command(file, recipients, output),
        Commands::Decrypt {
            file,
            private_key,
        } => decrypt_command(file, private_key),
        Commands::Rotate {
            file,
            private_key,
            recipients,
            output,
        } => rotate_command(file, private_key, recipients, output),
    };

    if let Err(err) = result {
        eprintln!("Error: {err}");
        process::exit(1);
    }
}

fn check_command(
    source: SourceArgs,
    schema: Option<PathBuf>,
    strict: bool,
    policy: Option<PathBuf>,
) -> Result<(), String> {
    let pairs = load_pairs_for_source(&source).map_err(|err| err.to_string())?;
    let target = describe_target(source.file.as_deref(), source.environment.as_deref());
    let schema_path = schema.or_else(|| existing_path(".env.schema"));

    if let Some(schema_path) = schema_path {
        let schema = dotenvpp::schema_from_path(&schema_path).map_err(|err| err.to_string())?;
        let report = schema.validate_pairs(&pairs);
        print_schema_report(&target, &report);
        if report.has_errors() {
            return Err(format!("{target}: schema validation failed"));
        }
    } else {
        println!(
            "{target} - {} variable{} parsed successfully",
            pairs.len(),
            if pairs.len() == 1 {
                ""
            } else {
                "s"
            }
        );
    }

    if strict {
        let report = evaluate_policy(&pairs, policy.as_deref()).map_err(|err| err.to_string())?;
        print_policy_report(&report);
        if report.has_errors() {
            return Err(format!("{target}: strict policy check failed"));
        }
    }

    Ok(())
}

fn run_command(
    source: SourceArgs,
    encrypted: bool,
    private_key: Option<String>,
    command: Vec<String>,
) -> Result<(), String> {
    match load_and_run(
        source.file.as_deref(),
        source.environment.as_deref(),
        encrypted,
        private_key,
        &command,
    ) {
        Ok(status) => exit_from_status(status),
        Err(RunError::MissingCommand) => Err("No command specified".to_owned()),
        Err(RunError::Load(err)) => Err(format!("Failed to load environment: {err}")),
        Err(RunError::Execute {
            program,
            source,
        }) => Err(format!("Failed to execute {program}: {source}")),
    }
}

fn schema_command(command: SchemaCommand) -> Result<(), String> {
    match command {
        SchemaCommand::Init {
            file,
            output,
        } => {
            let generated =
                dotenvpp::infer_schema_from_path(&file).map_err(|err| err.to_string())?;
            write_or_print(output.as_deref(), &generated)
        }
        SchemaCommand::Example {
            schema,
            output,
        } => {
            let schema = dotenvpp::schema_from_path(schema).map_err(|err| err.to_string())?;
            write_or_print(output.as_deref(), &schema.generate_example())
        }
        SchemaCommand::Docs {
            schema,
            output,
        } => {
            let schema = dotenvpp::schema_from_path(schema).map_err(|err| err.to_string())?;
            write_or_print(output.as_deref(), &schema.generate_docs())
        }
        SchemaCommand::JsonSchema {
            schema,
            output,
        } => {
            let schema = dotenvpp::schema_from_path(schema).map_err(|err| err.to_string())?;
            write_or_print(output.as_deref(), &schema.to_json_schema_string())
        }
    }
}

fn lint_command(source: SourceArgs, policy: Option<PathBuf>) -> Result<(), String> {
    let pairs = load_pairs_for_source(&source).map_err(|err| err.to_string())?;
    let report = evaluate_policy(&pairs, policy.as_deref()).map_err(|err| err.to_string())?;
    print_policy_report(&report);
    if report.violations.is_empty() {
        println!("No policy violations found");
    }
    Ok(())
}

fn keygen_command() -> Result<(), String> {
    let keypair = dotenvpp::crypto::keygen().map_err(|err| err.to_string())?;
    let output = serde_json::to_string_pretty(&keypair).map_err(|err| err.to_string())?;
    println!("{output}");
    Ok(())
}

fn encrypt_command(
    file: PathBuf,
    recipients: Vec<String>,
    output: Option<PathBuf>,
) -> Result<(), String> {
    let encrypted =
        dotenvpp::encrypt_path_to_string(file, &recipients).map_err(|err| err.to_string())?;
    write_or_print(output.as_deref(), &encrypted)
}

fn decrypt_command(file: PathBuf, private_key: Option<String>) -> Result<(), String> {
    let private_key = private_key_from_arg_or_env(private_key)?;
    let input = fs::read_to_string(file).map_err(|err| err.to_string())?;
    let pairs = dotenvpp::decrypt_env_str(&input, &private_key).map_err(|err| err.to_string())?;
    for pair in pairs {
        println!("{}={}", pair.key, pair.value);
    }
    Ok(())
}

fn rotate_command(
    file: PathBuf,
    private_key: Option<String>,
    recipients: Vec<String>,
    output: Option<PathBuf>,
) -> Result<(), String> {
    let private_key = private_key_from_arg_or_env(private_key)?;
    let input = fs::read_to_string(file).map_err(|err| err.to_string())?;
    let rotated = dotenvpp::crypto::rotate_str(&input, &private_key, &recipients)
        .map_err(|err| err.to_string())?;
    write_or_print(output.as_deref(), &rotated)
}

fn load_pairs_for_source(source: &SourceArgs) -> dotenvpp::Result<Vec<dotenvpp::EnvPair>> {
    match source.file.as_deref() {
        Some(path) => dotenvpp::from_path_iter(path).map(|pairs| pairs.collect()),
        None => dotenvpp::from_layered_env(source.environment.as_deref()),
    }
}

fn load_and_run(
    file: Option<&Path>,
    environment: Option<&str>,
    encrypted: bool,
    private_key: Option<String>,
    command: &[String],
) -> Result<ExitStatus, RunError> {
    if command.is_empty() {
        return Err(RunError::MissingCommand);
    }

    if encrypted {
        let Some(path) = file else {
            return Err(RunError::Load(dotenvpp::Error::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "--encrypted requires --file",
            ))));
        };
        let private_key = private_key_from_arg_or_env(private_key)
            .map_err(|err| RunError::Load(dotenvpp::Error::NotPresent(err)))?;
        dotenvpp::load_encrypted_path(path, &private_key, false).map_err(RunError::Load)?;
    } else {
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
    }

    let program = &command[0];
    let args = &command[1..];
    let status = Command::new(program).args(args).status().map_err(|source| RunError::Execute {
        program: program.clone(),
        source,
    })?;

    Ok(status)
}

fn evaluate_policy(
    pairs: &[dotenvpp::EnvPair],
    policy_path: Option<&Path>,
) -> Result<dotenvpp::policy::PolicyReport, dotenvpp::Error> {
    let policy = match policy_path {
        Some(path) => dotenvpp::policy_from_path(path)?,
        None => match existing_path(".env.policy") {
            Some(path) => dotenvpp::policy_from_path(path)?,
            None => dotenvpp::policy::standard_security_policy(),
        },
    };
    Ok(dotenvpp::evaluate_policy_for_pairs(pairs, &policy))
}

fn print_schema_report(target: &str, report: &dotenvpp::schema::ValidationReport) {
    println!(
        "{target} - {} typed variable{} validated",
        report.entries.len(),
        if report.entries.len() == 1 {
            ""
        } else {
            "s"
        }
    );
    for diagnostic in &report.diagnostics {
        println!(
            "{:?}: {}{}",
            diagnostic.severity,
            diagnostic.key.as_ref().map(|key| format!("{key}: ")).unwrap_or_default(),
            diagnostic.message
        );
    }
}

fn print_policy_report(report: &dotenvpp::policy::PolicyReport) {
    for violation in &report.violations {
        println!("{:?}: {} - {}", violation.severity, violation.rule, violation.message);
    }
}

fn private_key_from_arg_or_env(private_key: Option<String>) -> Result<String, String> {
    match private_key {
        Some(private_key) => Ok(private_key),
        None => {
            env::var("DOTENV_PRIVATE_KEY").map_err(|_| "DOTENV_PRIVATE_KEY is required".to_owned())
        }
    }
}

fn write_or_print(output: Option<&Path>, content: &str) -> Result<(), String> {
    if let Some(path) = output {
        fs::write(path, content).map_err(|err| err.to_string())?;
    } else {
        println!("{content}");
    }
    Ok(())
}

fn existing_path(path: &str) -> Option<PathBuf> {
    let path = PathBuf::from(path);
    path.exists().then_some(path)
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
