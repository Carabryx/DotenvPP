/// Load the default layered stack (.env + .env.local).
fn main() -> Result<(), dotenvpp::Error> {
    let pairs = dotenvpp::from_layered_env(None)?;

    println!("Default layered config ({} vars):\n", pairs.len());
    for p in &pairs {
        println!("  {:20} = {}", p.key, p.value);
    }

    Ok(())
}
