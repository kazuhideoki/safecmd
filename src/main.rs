use clap::Parser;
use std::path::PathBuf;

/// Move the specified file to the system trash.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Allow removing empty directories
    #[arg(short = 'd')]
    allow_dir: bool,
    /// Recursively remove directories
    #[arg(short = 'r')]
    recursive: bool,
    /// Paths to files or directories to trash
    path: Vec<PathBuf>,
}

fn main() {
    let args = Args::parse();
    let mut exit_code = 0;

    for path in &args.path {
        match std::fs::metadata(path) {
            Ok(meta) => {
                if meta.is_dir() {
                    if !args.allow_dir && !args.recursive {
                        eprintln!("safecmd: {}: is a directory", path.display());
                        exit_code = 1;
                        continue;
                    }

                    if args.recursive {
                        if let Err(e) = trash::delete(path) {
                            eprintln!(
                                "safecmd: failed to remove '{}': {}",
                                path.display(),
                                e
                            );
                            exit_code = 1;
                        }
                        continue;
                    }

                    match std::fs::read_dir(path) {
                        Ok(mut entries) => {
                            if entries.next().is_none() {
                                if let Err(e) = trash::delete(path) {
                                    eprintln!(
                                        "safecmd: failed to remove '{}': {}",
                                        path.display(),
                                        e
                                    );
                                    exit_code = 1;
                                }
                            } else {
                                eprintln!("safecmd: {}: Directory not empty", path.display());
                                exit_code = 1;
                            }
                        }
                        Err(e) => {
                            eprintln!(
                                "safecmd: cannot access '{}': {}",
                                path.display(),
                                e
                            );
                            exit_code = 1;
                        }
                    }
                } else if let Err(e) = trash::delete(path) {
                    eprintln!(
                        "safecmd: failed to remove '{}': {}",
                        path.display(),
                        e
                    );
                    exit_code = 1;
                }
            }
            Err(e) => {
                eprintln!("safecmd: cannot remove '{}': {}", path.display(), e);
                exit_code = 1;
            }
        }
    }

    std::process::exit(exit_code);
}
