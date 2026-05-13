use std::env;
use std::path::Path;
use qrd_cli::{convert_placeholder, inspect_file, inspect_footer_only, inspect_json, keygen_placeholder, verify_file};

fn print_help() {
    println!("QRD CLI");
    println!("Commands:");
    println!("  qrd-inspect <file>");
    println!("  qrd-inspect --schema <file>");
    println!("  qrd-inspect --footer <file>");
    println!("  qrd-inspect --json <file>");
    println!("  qrd-verify <file>");
    println!("  qrd-convert <mode> <input> <output>");
    println!("  qrd-keygen <mode>");
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

    let command = args.get(1).map(String::as_str).unwrap_or("");
    let result = match command {
        "qrd-inspect" => {
            if args.len() < 3 {
                Err("qrd-inspect requires a file path".into())
            } else if args.get(2).map(String::as_str) == Some("--schema") {
                if args.len() < 4 {
                    Err("qrd-inspect --schema requires a file path".into())
                } else {
                    inspect_file(Path::new(&args[3])).map(|output| print!("{output}"))
                }
            } else if args.get(2).map(String::as_str) == Some("--footer") {
                if args.len() < 4 {
                    Err("qrd-inspect --footer requires a file path".into())
                } else {
                    inspect_footer_only(Path::new(&args[3])).map(|output| print!("{output}"))
                }
            } else if args.get(2).map(String::as_str) == Some("--json") {
                if args.len() < 4 {
                    Err("qrd-inspect --json requires a file path".into())
                } else {
                    inspect_json(Path::new(&args[3])).map(|output| print!("{output}"))
                }
            } else {
                inspect_file(Path::new(&args[2])).map(|output| print!("{output}"))
            }
        }
        "qrd-verify" => {
            if args.len() < 3 {
                Err("qrd-verify requires a file path".into())
            } else if args.get(2).map(String::as_str) == Some("--ecc") {
                if args.len() < 4 {
                    Err("qrd-verify --ecc requires a file path".into())
                } else {
                    verify_file(Path::new(&args[3])).map(|output| print!("{output}"))
                }
            } else if args.get(2).map(String::as_str) == Some("--signature") {
                if args.len() < 4 {
                    Err("qrd-verify --signature requires a file path".into())
                } else {
                    verify_file(Path::new(&args[3])).map(|output| print!("{output}"))
                }
            } else if args.get(2).map(String::as_str) == Some("--auth-tags") {
                if args.len() < 4 {
                    Err("qrd-verify --auth-tags requires a file path".into())
                } else {
                    verify_file(Path::new(&args[3])).map(|output| print!("{output}"))
                }
            } else {
                verify_file(Path::new(&args[2])).map(|output| print!("{output}"))
            }
        }
        "qrd-convert" => {
            if args.len() < 5 {
                Err("qrd-convert requires mode, input, and output".into())
            } else {
                convert_placeholder(&args[2], Path::new(&args[3]), Path::new(&args[4]))
                    .map(|output| print!("{output}"))
            }
        }
        "qrd-keygen" => {
            if args.len() < 3 {
                Err("qrd-keygen requires a mode".into())
            } else {
                keygen_placeholder(&args[2]).map(|output| print!("{output}"))
            }
        }
        _ => Err("unknown command".into()),
    };

    if let Err(error) = result {
        eprintln!("{error}");
        std::process::exit(1);
    }
}