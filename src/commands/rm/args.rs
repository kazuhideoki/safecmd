use clap::Parser;
use std::path::PathBuf;

/// Move the specified file to the system trash.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// Allow removing empty directories
    #[arg(short = 'd')]
    pub allow_dir: bool,
    /// Force removal without prompting, ignore non-existent files
    #[arg(short = 'f')]
    pub force: bool,
    /// Recursively remove directories
    #[arg(short = 'r')]
    pub recursive: bool,
    /// Paths to files or directories to trash
    pub path: Vec<PathBuf>,
}
