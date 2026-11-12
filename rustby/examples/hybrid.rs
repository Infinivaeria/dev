fn main() {
    println!("Hello from Rust");
    match embedded::run() {
        Ok(code) => std::process::exit(code),
        Err(error) => {
            eprintln!("Ruby failed: {error}");
            std::process::exit(1);
        }
    }
}