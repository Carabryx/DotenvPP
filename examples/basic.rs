//! Basic in-crate example.
//!
//! Run with: `cargo run --example basic`

fn main() -> Result<(), dotenvpp::Error> {
    // Parse from a reader (no file needed for this example).
    let env_content = b"APP_NAME=dotenvpp\nAPP_PORT=8080\nDEBUG=true";
    let pairs = dotenvpp::from_read(&env_content[..])?;

    println!("🔧 DotenvPP v{}", dotenvpp::version());
    println!("📋 Parsed {} variables:\n", pairs.len());

    for pair in &pairs {
        println!("  {} = {}", pair.key, pair.value);
    }

    Ok(())
}
