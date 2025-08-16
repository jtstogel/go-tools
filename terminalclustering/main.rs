mod sgf;
mod config;

use anyhow::Ok;
use clap::Parser;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(long)]
    config: String,

    #[arg(long)]
    model: String,

    #[arg(long)]
    human_model: Option<String>,

    #[arg(long)]
    game: String,

    #[arg(long, default_value = "1000")]
    max_visits: i32,

    #[arg(long)]
    sample_size: i32,

    #[arg(long)]
    output_dir: String,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    let stat = std::fs::metadata(&args.output_dir)
        .map_err(|err| anyhow::Error::msg(format!("Output directory inaccessible: {}", err)))?;
    if !stat.is_dir() {
        return Err(anyhow::Error::msg("--output-dir must be a directory"));
    }

    Ok(())
}
