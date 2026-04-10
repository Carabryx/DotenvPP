/// Load the full production stack:
/// .env < .env.production < .env.local < .env.production.local
fn main() -> Result<(), dotenvpp::Error> {
    let pairs = dotenvpp::from_layered_env(Some("production"))?;

    println!("Production config ({} vars):\n", pairs.len());
    for p in &pairs {
        println!("  {:20} = {}", p.key, p.value);
    }

    Ok(())
}
