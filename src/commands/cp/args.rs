use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "cp")]
#[command(about = "Safely copy files and directories", long_about = None)]
pub struct Args {
    /// 互換性のため `-f` を受理する（安全挙動は変更しない）
    #[arg(short = 'f')]
    pub force: bool,

    /// Copy directories recursively
    #[arg(short = 'R', short_alias = 'r', long = "recursive")]
    pub recursive: bool,

    /// Files to copy (source and target)
    #[arg(required = true, num_args = 2..)]
    pub files: Vec<String>,
}
