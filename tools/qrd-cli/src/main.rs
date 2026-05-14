use qrd_cli::{inspect_file, inspect_footer_only, inspect_json, inspect_schema};
use std::env;
use std::path::Path;

fn print_help() {
    println!("QRD Inspect CLI");
    println!("Usage:");
    println!("  qrd-inspect <file>");
    println!("  qrd-inspect --schema <file>");
    println!("  qrd-inspect --footer <file>");
    println!("  qrd-inspect --json <file>");
}

fn print_version() {
    println!("qrd-cli 0.1.0");
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() == 1 || args.iter().any(|arg| arg == "--help" || arg == "-h") {
        print_help();
        return;
    }
    if args.iter().any(|arg| arg == "--version" || arg == "-V") {
        print_version();
        return;
    }

    let result = if args.get(1).map(String::as_str) == Some("--help")
        || args.get(1).map(String::as_str) == Some("-h")
    {
        print_help();
        return;
    } else if args.get(1).map(String::as_str) == Some("--version")
        || args.get(1).map(String::as_str) == Some("-V")
    {
        print_version();
        return;
    } else {
        let inspect_flag = args.get(1).map(String::as_str);
        match inspect_flag {
            Some("--schema") => {
                if args.len() != 3 {
                    Err("qrd-inspect --schema requires a file path".into())
                } else {
                    inspect_schema(Path::new(&args[2])).map(|output| print!("{output}"))
                }
            }
            Some("--footer") => {
                if args.len() != 3 {
                    Err("qrd-inspect --footer requires a file path".into())
                } else {
                    inspect_footer_only(Path::new(&args[2])).map(|output| print!("{output}"))
                }
            }
            Some("--json") => {
                if args.len() != 3 {
                    Err("qrd-inspect --json requires a file path".into())
                } else {
                    inspect_json(Path::new(&args[2])).map(|output| print!("{output}"))
                }
            }
            Some(file_path) => inspect_file(Path::new(file_path)).map(|output| print!("{output}")),
            None => Err("qrd-inspect requires a file path".into()),
        }
    };

    if let Err(error) = result {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
