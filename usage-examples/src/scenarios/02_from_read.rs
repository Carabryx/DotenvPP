/// Parse .env content from a string — useful for embedded configs or testing.
fn main() -> Result<(), dotenvpp::Error> {
    let config = b"HOST=localhost\nPORT=8080\nURL=http://${HOST}:${PORT}/api";

    let pairs = dotenvpp::from_read(&config[..])?;

    for p in &pairs {
        println!("{} = {}", p.key, p.value);
    }

    Ok(())
}
