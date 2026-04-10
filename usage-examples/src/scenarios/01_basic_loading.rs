/// Load .env and use variables — the most common usage.
fn main() -> Result<(), dotenvpp::Error> {
    dotenvpp::load()?;

    println!("App: {}", dotenvpp::var("APP_NAME")?);
    println!("Port: {}", dotenvpp::var("APP_PORT")?);
    println!("Debug: {}", dotenvpp::var("APP_DEBUG")?);
    println!("DB URL: {}", dotenvpp::var("DB_URL")?);

    Ok(())
}
