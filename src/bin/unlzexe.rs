use std::path::PathBuf;
use std::process;

fn usage(prog: &str) {
    eprintln!("Usage: {prog} <input.exe> [output.exe]");
    eprintln!();
    eprintln!("Decompresses an LZEXE 0.90/0.91/0.91e packed DOS EXE.");
    eprintln!("If output is omitted the decompressed file overwrites the input.");
    eprintln!();
    eprintln!("Exit codes: 0 = success, 1 = error, 2 = file was not LZEXE-compressed");
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let prog = args[0].as_str();

    let (input, output) = match args.len() {
        2 => {
            let i = PathBuf::from(&args[1]);
            let o = i.clone();
            (i, o)
        }
        3 => (PathBuf::from(&args[1]), PathBuf::from(&args[2])),
        _ => {
            usage(prog);
            process::exit(1);
        }
    };

    if args[1] == "--help" || args[1] == "-h" {
        usage(prog);
        process::exit(0);
    }

    let data = std::fs::read(&input).unwrap_or_else(|e| {
        eprintln!("error: cannot read {}: {e}", input.display());
        process::exit(1);
    });

    if !lzexe::is_compressed(&data) {
        eprintln!("{}: not compressed with LZEXE", input.display());
        process::exit(2);
    }

    let decompressed = lzexe::decompress(&data).unwrap_or_else(|e| {
        eprintln!("error: decompression failed: {e}");
        process::exit(1);
    });

    std::fs::write(&output, &decompressed).unwrap_or_else(|e| {
        eprintln!("error: cannot write {}: {e}", output.display());
        process::exit(1);
    });

    println!(
        "{} -> {} ({} -> {} bytes)",
        input.display(),
        output.display(),
        data.len(),
        decompressed.len()
    );
}
