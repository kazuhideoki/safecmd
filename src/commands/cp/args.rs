use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "cp")]
#[command(about = "Safely copy files and directories", long_about = None)]
pub struct Args {
    /// Files to copy (source and target)
    #[arg(required = true, num_args = 2..)]
    pub files: Vec<String>,
}
