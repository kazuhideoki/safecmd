use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "mv")]
#[command(about = "Safely move files and directories", long_about = None)]
pub struct Args {
    /// 互換性のため `-f` を受理する（安全挙動は変更しない）
    #[arg(short = 'f')]
    pub force: bool,

    /// 既存ファイルを上書きせずにスキップする
    #[arg(short = 'n')]
    pub no_clobber: bool,

    /// Files to move (source and target)
    #[arg(required = true, num_args = 2..)]
    pub files: Vec<String>,
}
