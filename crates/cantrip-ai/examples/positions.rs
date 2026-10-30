//! Encoded positions from the scenario corpus, for a model's parity references (`ai/cantrip_ai/export.py
//! --positions`): every `every`-th scenario's game recorded in Full mode, and up to `per` positions of each token bucket
//! taken from them, spread over the corpus, in `positions.bin`'s format (`cantrip_ai::eval::reference`).
//!
//!     cargo run --profile cards -p cantrip-ai --example positions -- OUT [every=40] [per=64]

use std::path::{Path, PathBuf};

use cantrip_ai::eval::reference::write_positions;
use cantrip_ai::record::{Dealt, Ending, record, table};
use cantrip_obs::InfoMode;
use cantrip_obs::encode::BUCKETS;
use cantrip_script::GameScript;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let out = PathBuf::from(args.first().expect("an output file"));
    let every: usize = args.get(1).map_or(40, |a| a.parse().unwrap());
    let per: usize = args.get(2).map_or(64, |a| a.parse().unwrap());
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut all: Vec<PathBuf> = std::fs::read_dir(root.join("scenarios"))
        .unwrap()
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|e| e == "cantrip"))
        .collect();
    all.sort();
    let sample: Vec<PathBuf> = all.into_iter().step_by(every).collect();
    let positions = std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(move || {
            let mut by_bucket: Vec<Vec<Vec<u8>>> = vec![Vec::new(); BUCKETS.len()];
            for path in &sample {
                let Ok(script) = GameScript::load(path) else {
                    continue;
                };
                let Ok(result) = cantrip_script::run(&script) else {
                    continue;
                };
                let game = result.game;
                let names = (0..game.state.players.len())
                    .map(|i| format!("Seat{}", i + 1))
                    .collect();
                let dealt = Dealt {
                    seed: 0,
                    names,
                    decks: Vec::new(),
                    auto_start: false,
                    ..Dealt::default()
                };
                let ending = Ending::of(&game, !game.is_over());
                let (rec, _) = record(game, &dealt, &ending, InfoMode::Full, table());
                for s in rec.samples {
                    let b = BUCKETS
                        .iter()
                        .position(|&t| t == s.position[1] as usize)
                        .unwrap();
                    by_bucket[b].push(s.position);
                }
            }
            let mut out = Vec::new();
            for (b, ps) in by_bucket.into_iter().enumerate() {
                let step = (ps.len() / per).max(1);
                let taken: Vec<Vec<u8>> = ps.into_iter().step_by(step).take(per).collect();
                eprintln!("{} tokens: {} positions", BUCKETS[b], taken.len());
                out.extend(taken);
            }
            out
        })
        .unwrap()
        .join()
        .unwrap();
    std::fs::write(&out, write_positions(&positions)).unwrap();
    eprintln!("{} positions to {}", positions.len(), out.display());
}
