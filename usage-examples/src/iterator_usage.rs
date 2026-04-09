//! Iterator example — use `from_path_iter` to process variables
//! without setting them in the environment.

pub fn run() -> Result<(), dotenvpp::Error> {
    println!("┌─────────────────────────────────────┐");
    println!("│  Example 3: Iterator Usage           │");
    println!("└─────────────────────────────────────┘");

    // Read pairs WITHOUT setting env vars — useful for
    // filtering, transforming, or inspecting config.
    let pairs: Vec<_> = dotenvpp::from_path_iter(".env")?.collect();

    // Filter only feature flags.
    let features: Vec<_> = pairs.iter().filter(|p| p.key.starts_with("FEATURE_")).collect();

    println!("  🚩 Feature flags ({}):\n", features.len());
    for pair in &features {
        let status = if pair.value == "true" {
            "✅ enabled"
        } else {
            "❌ disabled"
        };
        println!("    {:<24} {status}", pair.key);
    }

    // Filter database vars.
    let db_vars: Vec<_> = pairs.iter().filter(|p| p.key.starts_with("DB_")).collect();

    println!("\n  🗄️  Database config ({}):\n", db_vars.len());
    for pair in &db_vars {
        println!("    {:<20} = {}", pair.key, pair.value);
    }

    // Show summary.
    let empty_count = pairs.iter().filter(|p| p.value.is_empty()).count();
    println!("\n  📊 Summary:");
    println!("    Total variables: {}", pairs.len());
    println!("    Feature flags:   {}", features.len());
    println!("    Database vars:   {}", db_vars.len());
    println!("    Empty values:    {empty_count}");

    Ok(())
}
