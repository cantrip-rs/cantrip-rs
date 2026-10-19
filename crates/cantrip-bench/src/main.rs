//! cantrip-bench: what the engine costs per decision, on games the random seat plays (docs/engine-design.md,
//! "Performance"). The numbers are reported the way the old C# engine's were measured, so the two compare.
//!
//!   cantrip-bench [--games N] [--seed N] [--deck PATH --deck PATH] [--copies N] [--scaling] [--forks]

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;
use std::time::Instant;

use cantrip_bench::{Random, load_deck, play};
use cantrip_engine::*;

/// Counts the bytes each thread allocates.
struct Counting;

thread_local! {
    static ALLOCATED: Cell<u64> = const { Cell::new(0) };
}

unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        ALLOCATED.with(|a| a.set(a.get() + layout.size() as u64));
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        ALLOCATED.with(|a| a.set(a.get() + new_size as u64));
        unsafe { System.realloc(ptr, layout, new_size) }
    }
}

#[global_allocator]
static GLOBAL: Counting = Counting;

fn allocated() -> u64 {
    ALLOCATED.with(|a| a.get())
}

fn arg<T: std::str::FromStr>(args: &[String], name: &str, fallback: T) -> T {
    args.iter().position(|a| a == name).and_then(|i| args.get(i + 1)).and_then(|v| v.parse().ok()).unwrap_or(fallback)
}

fn summary(values: &mut [f64]) -> String {
    if values.is_empty() {
        return "-".into();
    }
    values.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let at = |q: f64| values[((q * (values.len() - 1) as f64).round() as usize).min(values.len() - 1)];
    let mean = values.iter().sum::<f64>() / values.len() as f64;
    let f = |v: f64| if v >= 100.0 { format!("{v:.0}") } else if v >= 10.0 { format!("{v:.1}") } else { format!("{v:.2}") };
    format!("{} [{}-{}] ({})", f(at(0.5)), f(at(0.1)), f(at(0.9)), f(mean))
}

struct Table {
    decks: [Vec<String>; 2],
    seed: u64,
}

impl Table {
    /// Game `i`: the decks swap seats every other game, each pair dealt from the same shuffle, as BotLab seats them.
    fn deal(&self, i: usize) -> Game {
        let decks = if i % 2 == 0 {
            vec![self.decks[0].clone(), self.decks[1].clone()]
        } else {
            vec![self.decks[1].clone(), self.decks[0].clone()]
        };
        deal(&decks, self.seed + (i / 2) as u64, true, None).expect("decks of known cards")
    }

    fn policy(&self, i: usize) -> Random {
        Random::new(self.seed * 7919 + i as u64)
    }
}

const LIMIT: u32 = 200_000;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let games: usize = arg(&args, "--games", 256);
    let seed: u64 = arg(&args, "--seed", 1);
    let copies: usize = arg(&args, "--copies", 20);
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let named: Vec<PathBuf> =
        args.iter().enumerate().filter(|(_, a)| *a == "--deck").filter_map(|(i, _)| args.get(i + 1)).map(PathBuf::from).collect();
    let paths = if named.len() == 2 {
        [named[0].clone(), named[1].clone()]
    } else {
        [root.join("decks/spike/spike-rg.txt"), root.join("decks/spike/spike-ubw.txt")]
    };
    let table = Table { decks: [load_deck(&paths[0]), load_deck(&paths[1])], seed };
    let threads = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1);

    println!(
        "Native engine benchmark: random against random, {games} games, {} / {}, shuffles from {seed}{}.",
        paths[0].file_stem().unwrap().to_string_lossy(),
        paths[1].file_stem().unwrap().to_string_lossy(),
        if cfg!(feature = "staged") { ", staged card code" } else { "" }
    );
    println!("{threads} logical processors.");

    // Warm up.
    for i in 0..4.min(games) {
        let mut g = table.deal(i);
        play(&mut g, &mut table.policy(i), LIMIT);
    }

    // --- One thread ---------------------------------------------------------------------------------------------
    let mut per_game_ms = Vec::new();
    let mut per_game_mb = Vec::new();
    let mut per_game_decisions = Vec::new();
    let mut per_game_turns = Vec::new();
    let mut deal_ms = 0.0;
    let mut total_s = 0.0;
    let mut engine_s = 0.0;
    let mut total_decisions = 0u64;
    let mut total_bytes = 0u64;
    let mut unfinished = 0;
    let mut lengths = Vec::new();
    for i in 0..games {
        let t = Instant::now();
        let mut g = table.deal(i);
        deal_ms += t.elapsed().as_secs_f64() * 1000.0;
        let mut random = table.policy(i);
        let before = allocated();
        let t = Instant::now();
        let played = play(&mut g, &mut random, LIMIT);
        let took = t.elapsed().as_secs_f64();
        let bytes = allocated() - before;
        if !played.finished {
            unfinished += 1;
        }
        total_s += took;
        engine_s += played.engine.as_secs_f64();
        total_decisions += played.decisions as u64;
        total_bytes += bytes;
        per_game_ms.push(took * 1000.0);
        per_game_mb.push(bytes as f64 / 1e6);
        per_game_decisions.push(played.decisions as f64);
        per_game_turns.push(played.turns as f64);
        lengths.push(played.decisions);
    }
    println!();
    println!("Engine with the random seat, one thread:");
    println!(
        "  {} games ({unfinished} unfinished at {LIMIT} decisions): {:.1} games/s, {:.0} decisions/s",
        games,
        games as f64 / total_s,
        total_decisions as f64 / total_s
    );
    println!("  per game: {} ms; allocated {} MB", summary(&mut per_game_ms), summary(&mut per_game_mb));
    println!("  per game: {} turns, {} decisions", summary(&mut per_game_turns), summary(&mut per_game_decisions));
    println!(
        "  per decision: {:.2} us, {:.2} KB allocated (the seat's choosing included)",
        total_s * 1e6 / total_decisions as f64,
        total_bytes as f64 / total_decisions as f64 / 1024.0
    );
    println!(
        "  of which the engine answering (the seat's choosing apart): {:.2} us per decision, {:.0} decisions/s",
        engine_s * 1e6 / total_decisions as f64,
        total_decisions as f64 / engine_s
    );
    println!("  dealing a game: {:.3} ms", deal_ms / games as f64);

    // --- Every core ---------------------------------------------------------------------------------------------
    if args.iter().any(|a| a == "--scaling") {
        println!();
        println!("By threads (each thread count plays every game once):");
        let mut n = 1;
        while n <= threads {
            let t = Instant::now();
            let next = std::sync::atomic::AtomicUsize::new(0);
            let decisions = std::sync::atomic::AtomicU64::new(0);
            std::thread::scope(|scope| {
                for _ in 0..n {
                    scope.spawn(|| {
                        loop {
                            let i = next.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                            if i >= games {
                                break;
                            }
                            let mut g = table.deal(i);
                            let played = play(&mut g, &mut table.policy(i), LIMIT);
                            decisions.fetch_add(played.decisions as u64, std::sync::atomic::Ordering::Relaxed);
                        }
                    });
                }
            });
            let s = t.elapsed().as_secs_f64();
            println!(
                "  {n:2} threads: {:8.1} games/s, {:10.0} decisions/s",
                games as f64 / s,
                decisions.load(std::sync::atomic::Ordering::Relaxed) as f64 / s
            );
            n *= 2;
        }
    }

    // --- Copying, make and unmake -------------------------------------------------------------------------------
    println!();
    println!("Copying a game, on {} games at a quarter, half and three quarters of the way through:", copies.min(games));
    for at in [0.25, 0.5, 0.75] {
        let mut objects = Vec::new();
        let mut clone_us = Vec::new();
        let mut clone_kb = Vec::new();
        let mut hash_us = Vec::new();
        let mut unmake_us = Vec::new();
        for (i, &length) in lengths.iter().enumerate().take(copies) {
            let k = ((length as f64 * at) as u32).max(1);
            let mut g = table.deal(i);
            let mut random = table.policy(i);
            play(&mut g, &mut random, k);
            if g.is_over() {
                continue;
            }
            objects.push(g.state.objects.len() as f64);

            const TIMES: u32 = 1000;
            // A position: what a search thread copies to have a working state of its own.
            let _ = g.state.clone();
            let before = allocated();
            let copy = g.state.clone();
            clone_kb.push((allocated() - before) as f64 / 1024.0);
            drop(copy);
            let t = Instant::now();
            for _ in 0..TIMES {
                std::hint::black_box(g.state.clone());
            }
            clone_us.push(t.elapsed().as_secs_f64() * 1e6 / TIMES as f64);

            let t = Instant::now();
            for _ in 0..TIMES {
                let mut h = std::collections::hash_map::DefaultHasher::new();
                g.state.hash(&mut h);
                std::hint::black_box(h.finish());
            }
            hash_us.push(t.elapsed().as_secs_f64() * 1e6 / TIMES as f64);

            // Make and unmake: one answer, and back.
            let request = g.pending().cloned().expect("waiting");
            let reference = g.state.clone();
            let t = Instant::now();
            for _ in 0..TIMES {
                let mut r = Random::new(i as u64);
                let answer = cantrip_bench::decide(&g, &mut r, &request);
                let mark = g.checkpoint();
                g.answer(request.id, request.player, answer).expect("legal");
                g.rollback(mark);
            }
            unmake_us.push(t.elapsed().as_secs_f64() * 1e6 / TIMES as f64);
            assert!(g.state == reference, "make and unmake did not come back to the same position");
        }
        println!("  at {:.0}%: {} objects in the state", at * 100.0, summary(&mut objects));
        println!("    copy a position (state clone): {} us, {} KB", summary(&mut clone_us), summary(&mut clone_kb));
        println!("    state hash: {} us", summary(&mut hash_us));
        println!("    answer and come back (the game-log tree): {} us", summary(&mut unmake_us));
    }
    // --- A fork for a search ------------------------------------------------------------------------------------
    if args.iter().any(|a| a == "--forks") {
        println!();
        println!("Forking a game for a search, on {} games at the start of turns 1, 5 and 15:", copies.min(games));
        for turn in [1, 5, 15] {
            let mut entries = Vec::new();
            let mut state_us = Vec::new();
            let mut journal_us = Vec::new();
            let mut journal_kb = Vec::new();
            let mut game_us = Vec::new();
            let mut game_kb = Vec::new();
            let mut fork_us = Vec::new();
            let mut fork_kb = Vec::new();
            let mut tail = Vec::new();
            for i in 0..copies.min(games) {
                let mut g = table.deal(i);
                let mut random = table.policy(i);
                while g.state.turn.number < turn && !g.is_over() {
                    play(&mut g, &mut random, 1);
                }
                if g.is_over() {
                    continue;
                }
                entries.push(g.state.journal.entries.len() as f64);
                const TIMES: u32 = 200;
                let time = |f: &dyn Fn()| {
                    let t = Instant::now();
                    for _ in 0..TIMES {
                        f();
                    }
                    t.elapsed().as_secs_f64() * 1e6 / TIMES as f64
                };
                state_us.push(time(&|| drop(std::hint::black_box(g.state.clone()))));
                journal_us.push(time(&|| drop(std::hint::black_box(g.state.clone_with_journal()))));
                game_us.push(time(&|| drop(std::hint::black_box(g.clone()))));
                fork_us.push(time(&|| drop(std::hint::black_box(g.fork_for_search()))));
                let before = allocated();
                let copy = g.state.clone_with_journal();
                journal_kb.push((allocated() - before) as f64 / 1024.0);
                drop(copy);
                let before = allocated();
                let copy = g.clone();
                game_kb.push((allocated() - before) as f64 / 1024.0);
                drop(copy);
                let before = allocated();
                let copy = g.fork_for_search();
                fork_kb.push((allocated() - before) as f64 / 1024.0);
                tail.push(copy.state.journal.entries.len() as f64);
                drop(copy);
            }
            println!("  turn {turn}: {} journal entries", summary(&mut entries));
            println!("    state clone: {} us", summary(&mut state_us));
            println!("    state with its journal: {} us, {} KB", summary(&mut journal_us), summary(&mut journal_kb));
            println!("    whole game (journal, history, tree): {} us, {} KB", summary(&mut game_us), summary(&mut game_kb));
            println!("    fork for search (journal tail, history, own tree): {} us, {} KB, {} entries in the tail", summary(&mut fork_us), summary(&mut fork_kb), summary(&mut tail));
        }
    }

    // --- Card code that asks ---------------------------------------------------------------------------------
    println!();
    println!("Answering a question asked mid-resolution, from the position a scenario hands over at:");
    let scripts = root.join("scenarios");
    for (file, replace) in [
        ("presentation-duress-hands-over-an-opponents-cards.cantrip", None),
        (
            "liliana-of-the-veil-minus-two-has-the-target-player-sacrifice-a-creature-of-their-choice.cantrip",
            Some(("Bot2.Choose(Guerrilla Gorilla)", "[Handover Bot2.Choose]")),
        ),
    ] {
        let mut text = std::fs::read_to_string(scripts.join(file)).expect("scenario");
        if let Some((from, to)) = replace {
            text = text.replace(from, to);
        }
        // The expectations are of the finished scenario, not the handed-over position.
        let text = text.split("[Expect]").next().unwrap().to_string();
        let script = cantrip_script::GameScript::parse(&text, file).expect("parses");
        let mut g = cantrip_script::run(&script).map_err(|f| f.report).expect("reaches its handover").game;
        let request = g.pending().cloned().expect("a question");
        let RequestKind::ChooseCards { eligible, count, .. } = &request.kind else { panic!("a card choice") };
        let answer = Answer::Cards(eligible.iter().take(*count as usize).copied().collect());
        const TIMES: u32 = 100_000;
        let t = Instant::now();
        for _ in 0..TIMES {
            let mark = g.checkpoint();
            g.answer(request.id, request.player, answer.clone()).expect("legal");
            g.rollback(mark);
        }
        let us = t.elapsed().as_secs_f64() * 1e6 / TIMES as f64;
        println!("  {}: {us:.2} us to answer and come back", file.trim_end_matches(".cantrip"));
    }

    println!();
    println!("Summaries are median [10th-90th percentile] (mean).");
}
