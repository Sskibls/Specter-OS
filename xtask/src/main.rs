use anyhow::Result;

fn main() -> Result<()> {
    let command = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "help".to_string());
    match command.as_str() {
        "help" => print_help(),
        "ci" => println!("xtask: CI entrypoint placeholder"),
        "tree" => println!("xtask: repository tree command placeholder"),
        other => {
            println!("xtask: unknown command '{other}'");
            print_help();
        }
    }

    Ok(())
}

fn print_help() {
    println!("Usage: cargo run -p xtask -- <help|ci|tree>");
}
