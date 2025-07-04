use clap::Parser;
use safecmd::commands::rm::{self, args::Args};
use safecmd::config::Config;

/// Safe replacement for the `rm` command.
fn main() {
    let args = Args::parse();

    // Load configuration
    let config = match Config::load() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("rm: {e}");
            std::process::exit(1);
        }
    };

    // Check if current directory is allowed
    if !config.is_current_dir_allowed() {
        eprintln!("rm: current directory is not in the allowed directories list");
        std::process::exit(1);
    }

    let exit_code = rm::run(args, config);
    std::process::exit(exit_code);
}
