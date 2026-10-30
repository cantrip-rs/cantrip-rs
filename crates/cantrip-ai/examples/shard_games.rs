//! Each game in a directory's shards, by its seed, with a hash of its bytes (`shard::game_bytes`): what two self-play
//! runs are compared by, game by game, whatever shards and lanes the games fell into. The headers are printed less
//! their `commit`; a report's timing fields are not read.
//!
//!     cargo run --release -p cantrip-ai --example shard_games -- DIR [DIR ...]
//!
//! Two runs played the same games when their outputs are equal (`diff`).

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use cantrip_ai::shard::{self, game_bytes};
use cantrip_obs::encode::fnv1a64;

fn main() {
    let dirs: Vec<PathBuf> = std::env::args().skip(1).map(PathBuf::from).collect();
    if dirs.is_empty() {
        eprintln!("usage: shard_games DIR [DIR ...]");
        std::process::exit(2);
    }
    let mut headers = BTreeSet::new();
    let mut games: BTreeMap<u64, Vec<u64>> = BTreeMap::new();
    for dir in &dirs {
        for path in shard::shards(dir).unwrap_or_else(|e| panic!("{}: {e}", dir.display())) {
            let (mut header, read) =
                shard::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
            header.commit = String::new();
            // The index is named by `text_hash`; its names would fill the screen.
            header.cards = vec![format!("{} names", header.cards.len())];
            headers.insert(serde_json::to_string(&header).unwrap());
            for g in read {
                games
                    .entry(g.meta.seed)
                    .or_default()
                    .push(fnv1a64(&game_bytes(&g)));
            }
        }
    }
    for h in &headers {
        println!("header {h}");
    }
    for (seed, hashes) in &games {
        let list: Vec<String> = hashes.iter().map(|h| format!("{h:016x}")).collect();
        println!("game {seed} {}", list.join(" "));
    }
    eprintln!("{} games", games.len());
}
