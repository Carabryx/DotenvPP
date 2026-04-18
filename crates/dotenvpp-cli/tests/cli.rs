#![allow(clippy::unwrap_used)]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

fn cli_command() -> Command {
    Command::new(env!("CARGO_BIN_EXE_dotenvpp"))
}

struct TempDir {
    path: PathBuf,
}

impl TempDir {
    fn new() -> Self {
        let mut path = std::env::temp_dir();
        let nanos = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        path.push(format!("dotenvpp-cli-it-{}-{nanos}", std::process::id()));
        fs::create_dir(&path).unwrap();
        Self {
            path,
        }
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn write_file(path: &Path, contents: &str) {
    fs::write(path, contents).unwrap();
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

fn run_and_collect(command: &mut Command) -> Output {
    command.output().unwrap()
}

#[test]
fn check_counts_pairs_for_explicit_file() {
    let temp_dir = TempDir::new();
    let file = temp_dir.path.join("basic.env");
    write_file(&file, "A=1\nB=2\n");

    let output = run_and_collect(cli_command().arg("check").arg("--file").arg(&file));
    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("2 variables parsed successfully"));
}

#[test]
fn check_returns_parse_error_for_explicit_file() {
    let temp_dir = TempDir::new();
    let file = temp_dir.path.join("invalid.env");
    write_file(&file, "NOT_VALID\n");

    let output = run_and_collect(cli_command().arg("check").arg("--file").arg(&file));
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("missing `=` separator"));
}

#[test]
fn check_uses_layered_environment_selection() {
    let temp_dir = TempDir::new();
    write_file(&temp_dir.path.join(".env"), "VALUE=base\n");
    write_file(&temp_dir.path.join(".env.production"), "EXTRA=${VALUE}-prod\n");
    write_file(&temp_dir.path.join(".env.local"), "VALUE=local\n");

    let output = run_and_collect(
        cli_command()
            .arg("check")
            .arg("--env")
            .arg("production")
            .current_dir(&temp_dir.path),
    );

    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("2 variables parsed successfully"));
}

#[test]
fn run_rejects_missing_command() {
    let temp_dir = TempDir::new();
    let file = temp_dir.path.join("basic.env");
    write_file(&file, "A=1\n");

    let output = run_and_collect(cli_command().arg("run").arg("--file").arg(&file));
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("required"));
}

#[test]
fn run_returns_load_error_for_missing_file() {
    let output = run_and_collect(
        cli_command()
            .arg("run")
            .arg("--file")
            .arg("definitely-missing.env")
            .arg("--")
            .arg("cmd"),
    );

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("Failed to load"));
}

#[test]
fn run_injects_env_into_child_process() {
    let temp_dir = TempDir::new();
    let key = "DOTENVPP_CLI_TEST_VALUE";
    let expected = "from_file";
    let file = temp_dir.path.join("run.env");
    write_file(&file, &format!("{key}={expected}\n"));

    let mut command = cli_command();
    command.arg("run").arg("--file").arg(&file).arg("--").env_remove(key);

    for arg in env_probe_command(key, expected) {
        command.arg(arg);
    }

    let output = run_and_collect(&mut command);
    assert!(output.status.success(), "stderr: {}", String::from_utf8_lossy(&output.stderr));
}

#[test]
fn run_uses_layered_environment_when_env_is_selected() {
    let temp_dir = TempDir::new();
    let key = "DOTENVPP_CLI_LAYERED_VALUE";
    let expected = "from_production_local";

    write_file(&temp_dir.path.join(".env"), &format!("{key}=base\n"));
    write_file(&temp_dir.path.join(".env.production"), &format!("{key}=from_production\n"));
    write_file(&temp_dir.path.join(".env.local"), &format!("{key}=from_local\n"));
    write_file(&temp_dir.path.join(".env.production.local"), &format!("{key}={expected}\n"));

    let mut command = cli_command();
    command
        .arg("run")
        .arg("--env")
        .arg("production")
        .arg("--")
        .current_dir(&temp_dir.path)
        .env_remove(key);

    for arg in env_probe_command(key, expected) {
        command.arg(arg);
    }

    let output = run_and_collect(&mut command);
    assert!(output.status.success(), "stderr: {}", String::from_utf8_lossy(&output.stderr));
}

#[test]
fn run_returns_execute_error() {
    let temp_dir = TempDir::new();
    let key = "DOTENVPP_CLI_EXEC_ERR";
    let file = temp_dir.path.join("exec.env");
    write_file(&file, &format!("{key}=1\n"));

    let output = run_and_collect(
        cli_command()
            .arg("run")
            .arg("--file")
            .arg(&file)
            .arg("--")
            .arg("definitely-not-a-real-command"),
    );

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("Failed to execute"));
}

#[test]
fn cli_rejects_file_and_env_together() {
    let output = run_and_collect(
        cli_command()
            .arg("check")
            .arg("--file")
            .arg(".env")
            .arg("--env")
            .arg("production"),
    );

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("--file"));
    assert!(stderr.contains("--env"));
}

#[test]
fn schema_commands_generate_and_check() {
    let temp_dir = TempDir::new();
    let env_file = temp_dir.path.join("app.env");
    let schema_file = temp_dir.path.join(".env.schema");
    let example_file = temp_dir.path.join(".env.example");
    write_file(&env_file, "PORT=3000\nAPI_KEY=abcdefgh\n");

    let output = run_and_collect(
        cli_command()
            .arg("schema")
            .arg("init")
            .arg("--file")
            .arg(&env_file)
            .arg("--output")
            .arg(&schema_file),
    );
    assert!(output.status.success(), "stderr: {}", String::from_utf8_lossy(&output.stderr));
    assert!(fs::read_to_string(&schema_file).unwrap().contains("[vars.PORT]"));

    let output = run_and_collect(
        cli_command()
            .arg("schema")
            .arg("example")
            .arg("--schema")
            .arg(&schema_file)
            .arg("--output")
            .arg(&example_file),
    );
    assert!(output.status.success(), "stderr: {}", String::from_utf8_lossy(&output.stderr));
    assert!(fs::read_to_string(&example_file).unwrap().contains("PORT="));

    let output = run_and_collect(
        cli_command()
            .arg("check")
            .arg("--file")
            .arg(&env_file)
            .arg("--schema")
            .arg(&schema_file),
    );
    assert!(output.status.success(), "stderr: {}", String::from_utf8_lossy(&output.stderr));
    assert!(String::from_utf8_lossy(&output.stdout).contains("typed variable"));
}

#[test]
fn check_strict_reports_policy_errors() {
    let temp_dir = TempDir::new();
    let env_file = temp_dir.path.join("app.env");
    let policy_file = temp_dir.path.join(".env.policy");
    write_file(&env_file, "ENV=production\nLOG_LEVEL=debug\n");
    write_file(
        &policy_file,
        r#"
        [[rules]]
        name = "no-debug"
        condition = "ENV == 'production' && LOG_LEVEL == 'debug'"
        severity = "error"
        "#,
    );

    let output = run_and_collect(
        cli_command()
            .arg("check")
            .arg("--file")
            .arg(&env_file)
            .arg("--strict")
            .arg("--policy")
            .arg(&policy_file),
    );

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("no-debug"));
}

#[test]
fn keygen_encrypt_and_decrypt_roundtrip() {
    let temp_dir = TempDir::new();
    let env_file = temp_dir.path.join("app.env");
    let enc_file = temp_dir.path.join(".env.enc");
    write_file(&env_file, "SECRET=from_file\n");

    let keygen = run_and_collect(cli_command().arg("keygen"));
    assert!(keygen.status.success(), "stderr: {}", String::from_utf8_lossy(&keygen.stderr));
    let keypair: serde_json::Value = serde_json::from_slice(&keygen.stdout).unwrap();
    let public_key = keypair["public_key"].as_str().unwrap();
    let private_key = keypair["private_key"].as_str().unwrap();

    let encrypted = run_and_collect(
        cli_command()
            .arg("encrypt")
            .arg("--file")
            .arg(&env_file)
            .arg("--recipient")
            .arg(public_key)
            .arg("--output")
            .arg(&enc_file),
    );
    assert!(encrypted.status.success(), "stderr: {}", String::from_utf8_lossy(&encrypted.stderr));
    assert!(fs::read_to_string(&enc_file).unwrap().contains("dotenvpp.enc.v1"));

    let decrypted = run_and_collect(
        cli_command()
            .arg("decrypt")
            .arg("--file")
            .arg(&enc_file)
            .arg("--private-key")
            .arg(private_key),
    );
    assert!(decrypted.status.success(), "stderr: {}", String::from_utf8_lossy(&decrypted.stderr));
    assert!(String::from_utf8_lossy(&decrypted.stdout).contains("SECRET=from_file"));
}
