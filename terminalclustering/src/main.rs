mod katago;
mod sgf;

use anyhow::Ok;
use clap::Parser;
use futures::stream::{FuturesUnordered, StreamExt};
use std::time::Duration;
use tokio::time::sleep;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    config: String,

    #[arg(short, long)]
    model: String,

    #[arg(short, long)]
    game: String,

    #[arg(short, long)]
    playouts: i32,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    let game = sgf::load_sgf(&args.game)?;

    let stones = sgf::sgf_to_stones(&game)?;

    let config = katago::parse_config(std::fs::read_to_string(&args.config)?.as_str())?;
    println!("Using config: {config:?}");
    let kg = katago::KataGo::new("katago", &args.config, &args.model).await?;

    sleep(Duration::from_secs(3)).await;

    let mut futures = FuturesUnordered::new();
    let mut i = 0;
    let mut joined_i = 0;
    let mut games = vec![];
    let mut save = |game| -> anyhow::Result<()> {
        sgf::save_game_sgf(
            &game,
            format!(
                "/home/jtstogel/github/jtstogel/kataplay/terminalclustering/sgfs/outputs/{i}.sgf"
            )
            .as_str(),
        )?;
        i += 1;
        games.push(game);

        let joined = sgf::combine_sgfs(games.as_slice())?;
        sgf::save_game_sgf(
            &joined,
            format!(
                "/home/jtstogel/github/jtstogel/kataplay/terminalclustering/sgfs/outputs/joined_{joined_i}.sgf"
            ).as_str()
        )?;
        // Only write batches of 50 games so they can be loaded by OGS.
        if games.len() > 50 {
            games.clear();
            joined_i += 1;
        }
        Ok(())
    };

    for _ in 0..args.playouts {
        let kg = kg.clone();
        let stones = stones.clone();
        futures.push(async move { kg.run_game(stones).await });

        if futures.len() == config.num_analysis_threads {
            save(sgf::stones_to_sgf(&futures.next().await.unwrap()?)?)?;
        }
    }
    while !futures.is_empty() {
        save(sgf::stones_to_sgf(&futures.next().await.unwrap()?)?)?;
    }
    Ok(())
}
