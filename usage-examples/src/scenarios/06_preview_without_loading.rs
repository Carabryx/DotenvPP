/// Preview config from a file without setting env vars.
/// Useful for validation, diffing, or CI checks.
fn main() -> Result<(), dotenvpp::Error> {
    let pairs: Vec<_> = dotenvpp::from_path_iter(".env")?.collect();

    let db_vars: Vec<_> = pairs.iter().filter(|p| p.key.starts_with("DB_")).collect();
    println!("Database vars:");
    for p in &db_vars {
        println!("  {} = {}", p.key, p.value);
    }

    let features: Vec<_> = pairs.iter().filter(|p| p.key.starts_with("FEATURE_")).collect();
    println!("\nFeature flags:");
    for p in &features {
        let on = p.value == "true";
        println!(
            "  {} {}",
            if on {
                "✅"
            } else {
                "❌"
            },
            p.key
        );
    }

    Ok(())
}
