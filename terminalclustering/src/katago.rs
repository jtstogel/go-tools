use anyhow::{Result, anyhow};
use rand::distr::{Distribution, weighted::WeightedIndex};
use std::{
    collections::HashMap, process::Stdio, sync::{
        atomic::{AtomicU32, Ordering}, Arc
    }
};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    process::ChildStdin,
    sync::{Mutex, MutexGuard, oneshot},
};

#[derive(Debug, serde_derive::Serialize, serde_derive::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnalysisRequest {
    pub id: String,
    pub initial_stones: Vec<(String, String)>,
    pub moves: Vec<(String, String)>,
    pub rules: String,
    pub komi: f32,
    pub board_x_size: i32,
    pub board_y_size: i32,
}

#[derive(Debug, serde_derive::Serialize, serde_derive::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnalysisResponseRootInfo {
    pub current_player: String,
}

#[derive(Debug, serde_derive::Serialize, serde_derive::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnalysisResponseMoveInfo {
    #[serde(rename = "move")]
    pub mov: String,
    pub utility: f32,
    pub score_lead: f32,
}

#[derive(Debug, serde_derive::Serialize, serde_derive::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnalysisResponse {
    pub id: String,
    pub root_info: AnalysisResponseRootInfo,
    pub move_infos: Vec<AnalysisResponseMoveInfo>,
}

pub struct KataGo {
    /// Writable handle to the engine’s STDIN.
    stdin: Mutex<ChildStdin>,
    /// Monotonically increasing request id generator.
    next_id: AtomicU32,
    /// Map id → `oneshot::Sender` so the reader can wake up the correct caller.
    pending: Mutex<HashMap<String, oneshot::Sender<AnalysisResponse>>>,
}

fn pick_move(moves: &[AnalysisResponseMoveInfo]) -> anyhow::Result<&AnalysisResponseMoveInfo> {
    let best_move = moves
        .iter()
        .max_by_key(|&m| (m.utility * 1000.0).round() as i64)
        .ok_or_else(|| anyhow::Error::msg("no moves returned by KataGo"))?;
    if best_move.mov == "pass" {
        return Ok(best_move);
    }

    let score = best_move.score_lead;
    let dist = WeightedIndex::new(moves.iter().map(|mv| (mv.utility * 0.5).exp()))?;
    let mut rng = rand::rng();

    // Pick randomly according to utility among moves that are close to the same estimated score.
    for _ in 0..10 {
        let choice = &moves[dist.sample(&mut rng)];
        if (score - choice.score_lead) < 0.5 && choice.mov != "pass" {
            return Ok(choice);
        }
    }
    return Ok(best_move);
}

impl KataGo {
    pub async fn new(
        katago_bin: impl AsRef<str>,
        config: impl AsRef<str>,
        model: impl AsRef<str>,
    ) -> Result<Arc<Self>> {
        let mut child = tokio::process::Command::new(katago_bin.as_ref())
            .args([
                "analysis",
                "-config",
                config.as_ref(),
                "-model",
                model.as_ref(),
            ])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            // If you want stderr too, add `.stderr(Stdio::null())` or `inherit`.
            .spawn()
            .map_err(|e| anyhow!("Failed to spawn kataGo: {e}"))?;

        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| anyhow!("Could not capture stdin"))?;

        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| anyhow!("Could not capture stdout"))?;

        let wrapper = Arc::new(Self {
            stdin: Mutex::new(stdin),
            next_id: AtomicU32::new(1),
            pending: Mutex::new(HashMap::new()),
        });

        Self::spawn_reader(wrapper.clone(), stdout);

        Ok(wrapper)
    }

    /// Issue a single analysis request and wait for the final reply.
    pub async fn analyze(&self, moves: Vec<(String, String)>) -> Result<AnalysisResponse> {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst).to_string();

        let req = AnalysisRequest {
            id: id.clone(),
            initial_stones: vec![],
            moves: moves.clone(),
            rules: "tromp-taylor".into(),
            komi: 7.5,
            board_x_size: 19,
            board_y_size: 19,
        };
        let (tx, rx) = oneshot::channel();
        self.pending.lock().await.insert(id.clone(), tx);
        self.write_request(&req).await?;
        rx.await
            .map_err(|_| anyhow!("KataGo process ended before sending a response"))
    }

    async fn write_request(&self, req: &AnalysisRequest) -> Result<()> {
        let line = serde_json::to_string(req)?;
        let mut stdin: MutexGuard<'_, ChildStdin> = self.stdin.lock().await;
        stdin.write_all(line.as_bytes()).await?;
        stdin.write_all(b"\n").await?;
        stdin.flush().await?;
        Ok(())
    }

    fn spawn_reader(wrapper: Arc<Self>, stdout: tokio::process::ChildStdout) {
        tokio::spawn(async move {
            let mut lines = BufReader::new(stdout).lines();

            while let Ok(Some(line)) = lines.next_line().await {
                if let Ok(resp) = serde_json::from_str::<AnalysisResponse>(&line) {
                    let id = resp.id.clone();
                    if let Some(tx) = wrapper.pending.lock().await.remove(&id) {
                        let _ = tx.send(resp);
                    }
                }
            }

            let mut pending = wrapper.pending.lock().await;
            for (_id, tx) in pending.drain() {
                drop(tx);
            }
        });
    }

    pub async fn run_game(
        &self,
        initial_stones: Vec<(String, String)>,
    ) -> Result<Vec<(String, String)>> {
        let mut stones = initial_stones;
        loop {
            let analysis_result = self.analyze(stones.clone()).await?;
            let mv = pick_move(&analysis_result.move_infos)?;
            if mv.mov == "pass" {
                return Ok(stones);
            }
            let score_for_black = if analysis_result.root_info.current_player == "W" {
                -mv.score_lead
            } else {
                mv.score_lead
            };
            let score_str = if score_for_black > 0. {
                format!("B+{:.1}", score_for_black)
            } else {
                format!("W+{:.1}", -score_for_black)
            };
            println!("move {}: {} {}\t({})", stones.len(), analysis_result.root_info.current_player, mv.mov, score_str);
            stones.push((
                analysis_result.root_info.current_player.clone(),
                mv.mov.clone(),
            ));
        }
    }
}

impl Drop for KataGo {
    fn drop(&mut self) {
        if let Ok(mut locked) = self.stdin.try_lock() {
            let _ = locked.write_all(b"\n");
            let _ = locked.flush();
        }
    }
}

#[derive(Debug)]
pub struct Config {
    pub num_analysis_threads: usize,
}

pub fn parse_config(content: &str) -> Result<Config> {
    // Finds lines of the form:
    // myKey = myValue  # Optional comment
    let key_value_regex = regex::Regex::new("^(\\w+)\\s*=\\s*(\\w+)\\s*(?:#.*)?$").unwrap();
    let entries: HashMap<&str, &str> = content
        .lines()
        .filter_map(|line| {
            let captures = key_value_regex.captures(line)?;
            let key = captures.get(1)?;
            let value = captures.get(2)?;
            Some((key.as_str(), value.as_str()))
        })
        .collect();

    Ok(Config {
        num_analysis_threads: entries
            .get("numAnalysisThreads")
            .ok_or_else(|| anyhow::Error::msg("numAnalysisThreads is required"))?
            .parse()?,
    })
}
