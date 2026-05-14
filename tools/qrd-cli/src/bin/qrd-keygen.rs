use qrd_cli::generate_key;
use std::env;

fn print_help() {
    println!("qrd-keygen <mode>");
    println!("Key generation modes:");
    println!("  master   Generate a random 32-byte master key in hex");
    println!("  signing  Generate an Ed25519 signing keypair in hex");
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 || args.iter().any(|arg| arg == "--help" || arg == "-h") {
        print_help();
        return;
    }

    let mode = &args[1];
    match generate_key(mode) {
        Ok(output) => print!("{output}"),
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(1);
        }
    }
}
