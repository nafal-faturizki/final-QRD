use qrd_cli::verify_file;
use std::env;
use std::path::Path;

fn print_help() {
    println!("qrd-verify <file>");
    println!("Verifies QRD integrity and schema signature status.");
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 || args.iter().any(|arg| arg == "--help" || arg == "-h") {
        print_help();
        return;
    }

    let path = Path::new(&args[1]);
    match verify_file(path) {
        Ok(output) => print!("{output}"),
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(1);
        }
    }
}
