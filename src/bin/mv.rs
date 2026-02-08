use clap::Parser;
use safecmd::commands::mv::{self, args::Args};
use safecmd::config::Config;

/// Safe replacement for the `mv` command.
fn main() {
    let args = Args::parse();

    // 設定ファイルを読み込む
    let config = match Config::load() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("mv: {e}");
            std::process::exit(1);
        }
    };

    let (target, sources) = args.files.split_last().unwrap();
    let exit_code = mv::run(
        sources.to_vec(),
        target.clone(),
        args.force,
        args.no_clobber,
        config,
    );
    std::process::exit(exit_code);
}
