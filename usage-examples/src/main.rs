//! DotenvPP Usage Examples
//!
//! Demonstrates using `dotenvpp` as an external dependency with real `.env` files.

mod basic_loading;
mod custom_path;
mod iterator_usage;

fn main() -> Result<(), dotenvpp::Error> {
    println!("╔══════════════════════════════════════╗");
    println!("║     DotenvPP — Usage Examples        ║");
    println!("║     v{}                          ║", dotenvpp::version());
    println!("╚══════════════════════════════════════╝\n");

    basic_loading::run()?;
    println!();
    custom_path::run()?;
    println!();
    iterator_usage::run()?;

    println!("\n✅ All examples completed successfully!");
    Ok(())
}
