#![allow(clippy::unwrap_used)]

use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use dotenvpp::ConfigSchema;

struct TempFile {
    path: PathBuf,
}

impl TempFile {
    fn new(name: &str, contents: &str) -> Self {
        let mut path = std::env::temp_dir();
        let nanos = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        path.push(format!("dotenvpp-phase-test-{}-{nanos}-{name}", std::process::id()));
        fs::write(&path, contents).unwrap();
        Self {
            path,
        }
    }
}

impl Drop for TempFile {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

#[derive(dotenvpp::Schema)]
#[allow(dead_code)]
struct AppConfig {
    #[env(name = "PORT", required, default = 8080, description = "HTTP port", range = [1024, 65535])]
    port: u16,
    #[env(name = "API_KEY", required, secret, min_length = 8)]
    api_key: String,
    #[env(name = "FEATURES")]
    features: Vec<String>,
}

#[test]
fn derive_schema_validates_config() {
    let schema = AppConfig::schema();
    let pairs =
        dotenvpp::from_read(b"PORT=9000\nAPI_KEY=abcdefgh\nFEATURES=a,b\n".as_slice()).unwrap();

    let report = schema.validate_pairs(&pairs);
    assert!(report.is_ok(), "{report:?}");
    assert!(report.entries["API_KEY"].secret);
    assert!(schema.generate_example().contains("PORT=8080"));
}

#[test]
fn evaluated_read_computes_expression_values() {
    let pairs = dotenvpp::from_read_evaluated(
        b"CPU_COUNT=4\nMAX_WORKERS=${CPU_COUNT} * 2 + 1\nLOG_LEVEL=info\n".as_slice(),
    )
    .unwrap();

    let max = pairs.iter().find(|pair| pair.key == "MAX_WORKERS").unwrap();
    assert_eq!(max.value, "9");
}

#[test]
fn schema_policy_and_crypto_work_through_facade() {
    let env_file = TempFile::new(
        "app.env",
        "ENV=production\nLOG_LEVEL=info\nDATABASE_URL=postgres://db/app?sslmode=require\nAPI_KEY=abcdefgh\nPORT=8080\n",
    );
    let schema_file = TempFile::new(
        "app.schema",
        r#"
        [vars.ENV]
        type = "enum"
        values = ["development", "production"]
        required = true

        [vars.LOG_LEVEL]
        type = "enum"
        values = ["debug", "info", "warn"]
        required = true

        [vars.DATABASE_URL]
        type = "url"
        required = true

        [vars.API_KEY]
        type = "string"
        required = true
        secret = true
        min_length = 8

        [vars.PORT]
        type = "port"
        required = true
        "#,
    );
    let policy_file = TempFile::new(
        "app.policy",
        r#"
        [[rules]]
        name = "no-debug"
        condition = "ENV == 'production' && LOG_LEVEL == 'debug'"
        severity = "error"
        "#,
    );

    let validation =
        dotenvpp::validate_path_with_schema(&env_file.path, &schema_file.path).unwrap();
    assert!(validation.is_ok(), "{validation:?}");

    let policy = dotenvpp::evaluate_policy_for_path(&env_file.path, &policy_file.path).unwrap();
    assert!(policy.is_ok(), "{policy:?}");

    let keypair = dotenvpp::crypto::keygen().unwrap();
    let encrypted =
        dotenvpp::encrypt_path_to_string(&env_file.path, std::slice::from_ref(&keypair.public_key))
            .unwrap();
    let decrypted = dotenvpp::decrypt_env_str(&encrypted, &keypair.private_key).unwrap();
    assert!(decrypted.iter().any(|pair| pair.key == "API_KEY" && pair.value == "abcdefgh"));
}
