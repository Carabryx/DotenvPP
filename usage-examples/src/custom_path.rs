//! Custom path example — load from `.env.production` to show
//! loading environment-specific config files.

pub fn run() -> Result<(), dotenvpp::Error> {
    println!("┌─────────────────────────────────────┐");
    println!("│  Example 2: Custom Path Loading      │");
    println!("└─────────────────────────────────────┘");

    // Load the production .env file (overrides existing vars
    // from .env if they exist).
    let pairs = dotenvpp::from_path_override(".env.production")?;

    println!(
        "  📋 Loaded {} variables from .env.production:\n",
        pairs.len()
    );

    for pair in &pairs {
        println!("    {:<20} = {}", pair.key, pair.value);
    }

    // Show how values changed from the base .env.
    println!("\n  🔄 Note how production values differ:");
    println!("    APP_ENV changed to: production");
    println!("    APP_DEBUG changed to: false");
    println!("    LOG_LEVEL changed to: warn");

    Ok(())
}
