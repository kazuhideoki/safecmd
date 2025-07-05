use clap::Parser;
use safecmd::commands::cp::{self, args::Args};
use safecmd::config::Config;

fn main() {
    let args = Args::parse();

    // 設定ファイルを読み込む
    let config = match Config::load() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("cp: {e}");
            std::process::exit(1);
        }
    };

    // 現在のディレクトリが許可されているか確認
    if !config.is_current_dir_allowed() {
        eprintln!("cp: current directory is not in the allowed directories list");
        std::process::exit(1);
    }

    // cpコマンドを実行
    if args.files.len() < 2 {
        eprintln!(
            "cp: missing destination file operand after '{}'\n",
            args.files.first().unwrap_or(&String::from(""))
        );
        eprintln!("Try 'cp --help' for more information.");
        std::process::exit(1);
    }

    let (target, sources) = args.files.split_last().unwrap();
    let exit_code = cp::run(sources.to_vec(), target.clone(), args.recursive, config);
    std::process::exit(exit_code);
}
