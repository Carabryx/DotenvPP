/// How errors look when things go wrong.
fn main() {
    // Missing required var
    let err = dotenvpp::from_read(
        b"API_KEY=${SECRET:?API_KEY is required -- set it in .env.local}\n".as_slice(),
    );
    println!("Required var missing:\n  {}\n", err.unwrap_err());

    // Circular reference
    let err = dotenvpp::from_read(b"A=${B}\nB=${A}\n".as_slice());
    println!("Circular reference:\n  {}\n", err.unwrap_err());

    // Bad syntax
    let err = dotenvpp::from_read(b"BROKEN LINE WITHOUT EQUALS\n".as_slice());
    println!("Parse error:\n  {}\n", err.unwrap_err());

    // Missing file
    let err = dotenvpp::from_path("nonexistent.env");
    println!("Missing file:\n  {}", err.unwrap_err());
}
