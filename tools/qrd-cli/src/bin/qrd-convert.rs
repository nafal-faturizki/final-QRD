use qrd_cli::convert_file;
use std::env;
use std::path::Path;

fn print_help() {
    println!("qrd-convert <mode> <input> <output>");
    println!("Conversion modes:");
    println!("  csv          Convert CSV file into QRD container");
    println!("  parquet      Convert Parquet file into QRD container");
    println!("  qrd-to-csv   Extract QRD container back to CSV-like bytes");
    println!("  qrd-to-parquet Extract QRD container back to Parquet-like bytes");
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 4 || args.iter().any(|arg| arg == "--help" || arg == "-h") {
        print_help();
        return;
    }

    let mode = &args[1];
    let input = Path::new(&args[2]);
    let output = Path::new(&args[3]);

    match convert_file(mode, input, output) {
        Ok(output) => print!("{output}"),
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(1);
        }
    }
}
