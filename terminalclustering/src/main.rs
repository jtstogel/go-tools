use anyhow::Ok;
use futures::stream::{FuturesUnordered, StreamExt};
use std::time::Duration;
use tokio::time::sleep;

mod katago;
mod sgf;

const CONFIG: &str =
    "/home/jtstogel/github/jtstogel/kataplay/terminalclustering/configs/analysis.cfg";
const MODEL: &str = "/home/jtstogel/katago/models/b10c128-s1141046784-d204142634/model.txt.gz";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let game = sgf::load_sgf(
        "/home/jtstogel/github/jtstogel/kataplay/terminalclustering/configs/example.sgf",
    )?;
    let stones = sgf::sgf_to_stones(&game)?;

    let config = katago::parse_config(std::fs::read_to_string(CONFIG)?.as_str())?;
    println!("Using config: {config:?}");
    let kg = katago::KataGo::new("katago", CONFIG, MODEL).await?;

    sleep(Duration::from_secs(3)).await;

    let mut futures = FuturesUnordered::new();
    let mut i = 0;
    for _ in 0..512 {
        let kg = kg.clone();
        let stones = stones.clone();
        futures.push(async move { kg.run_game(stones).await });

        if futures.len() == config.num_analysis_threads {
            let stones: Vec<(String, String)> = futures.next().await.unwrap()?;
            sgf::write_as_sgf(
                &stones,
                format!(
                    "/home/jtstogel/github/jtstogel/kataplay/terminalclustering/configs/sgfs/{i}.sgf"
                )
                .as_str(),
            )?;
            i += 1;
        }
    }

    Ok(())
}
