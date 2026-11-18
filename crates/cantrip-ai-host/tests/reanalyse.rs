//! Reanalyse (`cantrip_ai_host::reanalyse`): self-played games with traces, chosen positions searched again and
//! written as shards. With the tiny model, a few short games at a small budget: the chosen positions are stood in again
//! exactly (each written sample's position is the self-play shard's), each is a search target with q values, and the
//! report and the chosen positions' file say what was done.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use cantrip_ai::shard::{self, ShardReader};
use cantrip_ai::{Evaluator, Prior, TractEvaluator, record::table};
use cantrip_ai_host::reanalyse::{self, Options};
use cantrip_ai_host::selfplay::{self, Pairing, Run};

fn tiny() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../cantrip-ai/tests/data/tiny")
}

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn pairing() -> Pairing {
    let deck = |n: &str| selfplay::load_deck(&root().join("decks").join(format!("{n}.txt"))).expect("the decklist");
    Pairing { names: ["rw-aggro".into(), "ub-reanimator".into()], lists: [deck("rw-aggro"), deck("ub-reanimator")] }
}

fn load() -> Result<Box<dyn Evaluator>, String> {
    Ok(Box::new(TractEvaluator::load(&tiny(), table()).map_err(|e| e.to_string())?))
}

#[test]
fn chosen_positions_are_searched_again_from_their_traces() {
    std::thread::Builder::new()
        .stack_size(cantrip_ai_host::STACK_SIZE)
        .spawn(|| {
            let base = std::env::temp_dir().join(format!("cantrip-reanalyse-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&base);
            let (games, out) = (base.join("selfplay"), base.join("reanalysed"));
            let prior = Prior::from_decks(&root().join("decks")).unwrap();
            let options = selfplay::Options {
                model: "tiny".into(),
                simulations: 8,
                full_simulations: 24,
                full_share: 0.25,
                sampled: 4,
                max_turns: 4,
                traces: true,
                ..selfplay::Options::default()
            };
            let run = Run {
                out: games.clone(),
                games: 4,
                limit: None,
                lanes: 2,
                seed: 41,
                pairings: vec![pairing()],
                producer: "selfplay:tiny@8".into(),
                shard_mb: 64,
                shard_games: 100,
                resume: false,
                govern: Default::default(),
                grant: None,
                host: None,
            };
            selfplay::run(&run, &options, &prior, load).unwrap();
            let o = Options { positions: 6, random_share: 0.5, simulations: 16, worlds: 1, batch: 4, ..Options::default() };
            let r = reanalyse::Run { dirs: vec![games.clone()], out: out.clone(), lanes: 2, producer: "reanalyse:tiny@16".into(), starts: None };
            let report = reanalyse::run(&r, &o, &prior, load).unwrap();
            assert_eq!(report["GamesTraced"], 4, "{report}");
            assert_eq!(report["Chosen"], 6, "{report}");
            assert_eq!(report["MismatchedPositions"], 0, "{report}");
            assert!(report["Searched"].as_u64().unwrap() > 0, "{report}");
            assert!(report["ByReason"]["random"].as_u64().unwrap() >= 3, "{report}");
            let chosen = std::fs::read_to_string(out.join("reanalyse-positions.jsonl")).unwrap();
            assert_eq!(chosen.lines().count(), 6);
            // The self-play shards' positions, by game, decision and sub-step.
            let mut original: HashMap<(u64, u32, u16), Vec<u8>> = HashMap::new();
            for path in shard::shards(&games).unwrap() {
                for g in shard::read(&path).unwrap().1 {
                    for s in g.samples {
                        original.insert((g.meta.seed, s.decision, s.step), s.position);
                    }
                }
            }
            let mut written = 0;
            for path in shard::shards(&out).unwrap() {
                let mut reader = ShardReader::open(&path).unwrap();
                assert_eq!(reader.header.producer, "reanalyse:tiny@16");
                while let Some(g) = reader.game().unwrap() {
                    assert!(g.meta.reanalysed);
                    assert!(g.meta.fast_decisions.is_empty());
                    for s in &g.samples {
                        assert!(g.meta.root_values.iter().any(|&(d, _)| d == s.decision), "only the searched decisions");
                        assert_eq!(original.get(&(g.meta.seed, s.decision, s.step)), Some(&s.position), "the replay is the game");
                        if s.searched {
                            assert_eq!(s.q.len(), s.visits.len());
                        }
                        written += 1;
                    }
                }
            }
            assert_eq!(report["Samples"], written, "{report}");
            assert!(written > 0);

            // Traces cut short (as a table's game's can be, `Trace::of_game`): only the positions they hold are chosen,
            // and the labels are the shard's.
            let (cut, out2) = (base.join("cut"), base.join("reanalysed-cut"));
            std::fs::create_dir_all(cut.join("traces")).unwrap();
            for path in shard::shards(&games).unwrap() {
                std::fs::copy(&path, cut.join(path.file_name().unwrap())).unwrap();
            }
            let mut held = HashMap::new();
            let mut lines = String::new();
            for (seed, mut t) in cantrip_ai_host::regret::load_traces(&games.join("traces")).unwrap() {
                t.steps.truncate(t.steps.len() / 2);
                t.complete = false;
                held.insert(seed, t.steps.len() as u32);
                lines += &format!("{}
", serde_json::to_string(&t).unwrap());
            }
            std::fs::write(cut.join("traces/cut.jsonl"), lines).unwrap();
            let r = reanalyse::Run { dirs: vec![cut.clone()], out: out2.clone(), lanes: 2, producer: "reanalyse:tiny@16".into(), starts: None };
            let report = reanalyse::run(&r, &o, &prior, load).unwrap();
            assert_eq!(report["MismatchedPositions"], 0, "{report}");
            assert!(report["Searched"].as_u64().unwrap() > 0, "{report}");
            let mut labels: HashMap<(u64, u32, u16), (i8, f32, bool)> = HashMap::new();
            for path in shard::shards(&games).unwrap() {
                for g in shard::read(&path).unwrap().1 {
                    for s in g.samples {
                        labels.insert((g.meta.seed, s.decision, s.step), (s.outcome, s.life_swing, s.dead_next_turn));
                    }
                }
            }
            for path in shard::shards(&out2).unwrap() {
                for g in shard::read(&path).unwrap().1 {
                    for s in &g.samples {
                        assert!(s.decision < held[&g.meta.seed], "a decision the cut trace holds");
                        assert_eq!(labels[&(g.meta.seed, s.decision, s.step)], (s.outcome, s.life_swing, s.dead_next_turn), "the shard's labels");
                    }
                }
            }
            let _ = std::fs::remove_dir_all(&base);
        })
        .unwrap()
        .join()
        .unwrap();
}
