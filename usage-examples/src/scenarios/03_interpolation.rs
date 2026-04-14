/// All the interpolation features in action.
fn main() -> Result<(), dotenvpp::Error> {
    let config = br#"
HOST=localhost
PORT=5432
DB_URL=postgres://${HOST}:${PORT}/mydb

# Default values
TIMEOUT=${CUSTOM_TIMEOUT:-30}
REGION=${AWS_REGION:-us-east-1}

# Alternative -- show "enabled" only if webhook is set
WEBHOOK_STATUS=${SLACK_WEBHOOK:+enabled}

# Literal dollar sign
PRICE=$$19.99

# Chained
BASE=${DB_URL}
FULL=${BASE}?sslmode=require
"#;

    let pairs = dotenvpp::from_read(&config[..])?;

    for p in &pairs {
        println!("{:20} = {}", p.key, p.value);
    }

    Ok(())
}
