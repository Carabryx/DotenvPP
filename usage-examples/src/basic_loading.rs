//! Basic loading example — load `.env` and access variables.
//!
//! No need for `use std::env` — dotenvpp provides `var()` directly.

pub fn run() -> Result<(), dotenvpp::Error> {
    println!("┌─────────────────────────────────────┐");
    println!("│  Example 1: Basic .env Loading       │");
    println!("└─────────────────────────────────────┘");

    // Load the .env file.
    let pairs = dotenvpp::from_path(".env")?;

    println!("  📋 Loaded {} variables from .env:\n", pairs.len());

    for pair in &pairs {
        println!("    {:<20} = {}", pair.key, pair.value);
    }

    // Access individual vars — no `use std::env` needed!
    println!("\n  🔍 Accessing variables directly:");
    println!("    APP_NAME = {}", dotenvpp::var("APP_NAME")?);
    println!("    APP_PORT = {}", dotenvpp::var("APP_PORT")?);

    Ok(())
}
