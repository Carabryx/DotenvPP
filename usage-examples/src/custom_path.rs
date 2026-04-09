//! Custom path example — load from `.env.production` manually.
//!
//! This demonstrates explicit file selection in Phase 0, not
//! built-in environment layering.

pub fn run() -> Result<(), dotenvpp::Error> {
    println!("┌─────────────────────────────────────┐");
    println!("│  Example 2: Custom Path Loading      │");
    println!("└─────────────────────────────────────┘");

    // Load the production .env file manually. This uses override
    // semantics against variables already loaded from `.env`, but
    // the caller still chooses the merge order explicitly.
    let pairs = dotenvpp::from_path_override(".env.production")?;

    println!("  📋 Loaded {} variables from .env.production:\n", pairs.len());

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
