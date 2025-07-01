use clap::Parser;
use std::path::PathBuf;

/// Move the specified file to the system trash.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Path to the file or directory to trash
    path: PathBuf,
}

fn main() {
    let args = Args::parse();

    if let Err(e) = trash::delete(&args.path) {
        eprintln!("failed to trash {}: {}", args.path.display(), e);
    }
}
