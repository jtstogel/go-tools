use anyhow::Ok;
use clap::Parser;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    config: String,

    #[arg(short, long)]
    model: String,

    #[arg(long)]
    human_model: Option<String>,

    #[arg(short, long)]
    game: String,

    #[arg(short, long)]
    playouts: i32,

    #[arg(short, long)]
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
