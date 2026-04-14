//! Mixed scenario tests — realistic workloads combining multiple features.

use crate::parser::parse;

#[test]
fn quote_styles() {
    let input = "A='single'\nB=\"double\"\nC=unquoted";
    let pairs = parse(input).unwrap();
    assert_eq!(pairs[0].value, "single");
    assert_eq!(pairs[1].value, "double");
    assert_eq!(pairs[2].value, "unquoted");
}

#[test]
fn with_comments_and_blanks() {
    let input = "# Header\n\nA=1\n# Middle\nB=2\n\n# End";
    let pairs = parse(input).unwrap();
    assert_eq!(pairs.len(), 2);
    assert_eq!(pairs[0].value, "1");
    assert_eq!(pairs[1].value, "2");
}

#[test]
fn export_and_regular() {
    let input = "export A=1\nB=2\nexport C=3";
    let pairs = parse(input).unwrap();
    assert_eq!(pairs.len(), 3);
    assert_eq!(pairs[0].value, "1");
    assert_eq!(pairs[1].value, "2");
    assert_eq!(pairs[2].value, "3");
}

#[test]
fn multiline_and_single_line() {
    let input = "A=simple\nB=\"multi\nline\"\nC=also_simple";
    let pairs = parse(input).unwrap();
    assert_eq!(pairs.len(), 3);
    assert_eq!(pairs[0].value, "simple");
    assert_eq!(pairs[1].value, "multi\nline");
    assert_eq!(pairs[2].value, "also_simple");
}

#[test]
fn realistic_env_file() {
    let input = "\
# Application configuration
APP_NAME=dotenvpp
APP_ENV=development
APP_DEBUG=true

# Database
DB_HOST=localhost
DB_PORT=5432
DB_NAME=\"my_application\"
DB_USER='admin'
DB_PASS=\"s3cr3t#pass\"

# API Keys
export API_KEY=abc123def456
export API_SECRET=\"multi-part
secret-value\"

# Misc
LOG_LEVEL=info # default to info
EMPTY_VAR=
";
    let pairs = parse(input).unwrap();
    assert_eq!(pairs.len(), 12);

    let find = |key: &str| -> &str {
        &pairs
            .iter()
            .find(|p| p.key == key)
            .expect("expected key to be present in parsed pairs")
            .value
    };
    assert_eq!(find("APP_NAME"), "dotenvpp");
    assert_eq!(find("DB_NAME"), "my_application");
    assert_eq!(find("DB_USER"), "admin");
    assert_eq!(find("DB_PASS"), "s3cr3t#pass");
    assert_eq!(find("API_KEY"), "abc123def456");
    assert_eq!(find("API_SECRET"), "multi-part\nsecret-value");
    assert_eq!(find("LOG_LEVEL"), "info");
    assert_eq!(find("EMPTY_VAR"), "");
}

#[test]
fn realistic_docker_compose_style() {
    let input = "COMPOSE_PROJECT_NAME=myproject\n\
                 MYSQL_ROOT_PASSWORD=\"p@ssw0rd!\"\n\
                 MYSQL_DATABASE=app_db\n\
                 REDIS_URL=redis://localhost:6379/0";
    let pairs = parse(input).unwrap();
    assert_eq!(pairs.len(), 4);

    let find = |key: &str| -> &str {
        &pairs
            .iter()
            .find(|p| p.key == key)
            .expect("expected key to be present in parsed pairs")
            .value
    };
    assert_eq!(find("COMPOSE_PROJECT_NAME"), "myproject");
    assert_eq!(find("MYSQL_ROOT_PASSWORD"), "p@ssw0rd!");
    assert_eq!(find("MYSQL_DATABASE"), "app_db");
    assert_eq!(find("REDIS_URL"), "redis://localhost:6379/0");
}

#[test]
fn consecutive_exports() {
    let input = "export A=1\nexport B=2\nexport C=3";
    let pairs = parse(input).unwrap();
    assert_eq!(pairs.len(), 3);
    assert_eq!(pairs[0].value, "1");
    assert_eq!(pairs[1].value, "2");
    assert_eq!(pairs[2].value, "3");
}

#[test]
fn error_mid_file() {
    let input = "A=1\nINVALID\nB=2";
    let err = parse(input).unwrap_err();
    assert!(matches!(
        err,
        crate::error::ParseError::MissingSeparator {
            line: 2,
            ..
        }
    ));
}

#[test]
fn comments_every_other_line() {
    let input = "# c1\nA=1\n# c2\nB=2\n# c3\nC=3";
    let pairs = parse(input).unwrap();
    assert_eq!(pairs.len(), 3);
    assert_eq!(pairs[0].value, "1");
    assert_eq!(pairs[1].value, "2");
    assert_eq!(pairs[2].value, "3");
}

#[test]
fn all_empty_values() {
    let input = "A=\nB=''\nC=\"\"";
    let pairs = parse(input).unwrap();
    assert_eq!(pairs.len(), 3);
    for p in &pairs {
        assert_eq!(p.value, "");
    }
}
