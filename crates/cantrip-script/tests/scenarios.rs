//! Every scenario of `scenarios/*.cantrip` against the engine, one test each (see build.rs). A scenario whose decks
//! name a card the engine does not have yet is skipped with a note; one that can run must pass - unless it carries a
//! `# Known bug:` marker, in which case it must fail (a panic counts), and passing is the error: the bug is fixed and
//! the marker has to come off. A scenario that passes is played again with its seats driven (`driven`), and must end
//! the same.

use std::path::{Path, PathBuf};

use cantrip_script::{GameScript, known_bug, run};

fn scripts() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../scenarios")
}

fn scenario(name: &str) {
    let path = scripts().join(format!("{name}.cantrip"));
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{name}: {e}"));
    let script = GameScript::load(&path).unwrap_or_else(|e| panic!("{name}: {e}"));
    let mut missing: Vec<String> = script.cards().into_iter().filter(|c| cantrip_engine::card(c).is_none()).collect();
    missing.sort();
    missing.dedup();
    if !missing.is_empty() {
        println!("{name}: skipped, the engine has no {}", missing.join(", "));
        return;
    }
    match known_bug(&text) {
        None => {
            let alone = match run(&script) {
                Ok(result) => result.dump.render(),
                Err(failure) => panic!("\n{}", failure.report),
            };
            // Driven seats change nothing: it ends the same when each seat answers what the script means as when the
            // runner answers alone.
            let through = driven(script);
            if through.as_ref() != Ok(&alone) {
                panic!("{name} ends differently with its seats driven: {}", through.err().unwrap_or_else(|| "the final state differs".into()));
            }
        }
        Some(bug) => {
            let outcome = std::panic::catch_unwind(|| run(&script).map(|_| ()));
            if let Ok(Ok(())) = outcome {
                panic!("{name} passes: the known bug is fixed ({bug}). Remove its `# Known bug:` line.");
            }
            println!("{name}: known bug, still failing ({bug})");
        }
    }
}

include!(concat!(env!("OUT_DIR"), "/scenarios.rs"));

/// The checker itself: a scenario with one expectation changed must fail.
#[test]
fn a_wrong_expectation_fails() {
    let path = scripts().join("duress-takes-the-named-card.cantrip");
    let text = std::fs::read_to_string(&path).unwrap().replace("Bot2.Graveyard = Counterspell", "Bot2.Graveyard = Lightning Bolt");
    let script = GameScript::parse(&text, "mutated").unwrap();
    let failure = run(&script).err().expect("a wrong expectation must fail");
    assert!(failure.message.contains("Bot2.Graveyard"), "{}", failure.message);
}

/// Plays a scenario through `Scripted` with every seat driven, each answering exactly what the script means: the way a
/// table plays it with browsers in the seats. Returns the final dump, or the failure.
fn driven(script: GameScript) -> Result<String, String> {
    use cantrip_engine::PlayerId;
    use cantrip_script::{Advance, Scripted};
    let seats: Vec<PlayerId> = (0..script.decks.len() as u8).map(PlayerId).collect();
    let mut table = Scripted::deal(script).map_err(|f| f.message)?;
    loop {
        match table.advance(&seats).map_err(|f| f.message)? {
            Advance::Ask(expected) => table.answered(expected.intent).map_err(|f| f.message)?,
            Advance::Check(_) => {}
            Advance::Done => return table.verdict().map(|dump| dump.render()).map_err(|f| f.message),
        }
    }
}

/// A driven seat that answers other than the script means fails at the line.
#[test]
fn a_driven_seat_answering_otherwise_fails_at_the_line() {
    use cantrip_engine::{Answer, PlayerId};
    use cantrip_script::{Advance, Intent, Scripted};
    let script = GameScript::load(&scripts().join("duress-takes-the-named-card.cantrip")).unwrap();
    let mut table = Scripted::deal(script).unwrap();
    loop {
        match table.advance(&[PlayerId(0)]).unwrap() {
            Advance::Ask(expected) if expected.line.is_some() => {
                let failure = table.answered(Intent::Answer(Answer::Pass)).err().expect("a pass where a line plays fails");
                assert!(failure.message.contains(expected.line.as_deref().unwrap()), "{}", failure.message);
                return;
            }
            Advance::Ask(expected) => table.answered(expected.intent).unwrap(),
            Advance::Check(_) => {}
            Advance::Done => panic!("no line was ever asked of Bot1"),
        }
    }
}

/// An "up to" slot that chose nothing is no target (CR 601.2c, 115.1): it keeps its place among the item's slots as
/// `Target::None`, and nothing becomes the target of the ability because of it. In The Theorist, Jace Beleren's -2 here the
/// first opponent's slot chose nothing and the second's chose the Sol Ring: the history holds one `BecameTarget`, for the Sol Ring.
#[test]
fn a_slot_that_chose_nothing_makes_no_became_target_event() {
    use cantrip_engine::*;
    let path = scripts().join("the-theorist-jace-beleren-minus-two-targets-only-the-second-opponent-in-a-three-player-game.cantrip");
    let script = GameScript::load(&path).unwrap();
    let result = run(&script).unwrap_or_else(|f| panic!("\n{}", f.report));
    let became: Vec<Target> = result
        .game
        .history()
        .all()
        .iter()
        .filter_map(|r| match r.event {
            Event::BecameTarget { target, .. } => Some(target),
            _ => None,
        })
        .collect();
    assert_eq!(became.len(), 1, "{became:?}");
    assert!(matches!(became[0], Target::Object(_)), "{became:?}");
}

/// CR 702.22h, 509.3a-b: Smuggler's Copter declared to block Benalish Hero blocks the Hero's band-mate Guerrilla Gorilla too.
/// The history holds a block of each - the Hero's declared, the Gorilla's through the band - so an ability that triggers on
/// each creature it blocks sees both, while "whenever this blocks" (the declared one) triggered once (the scenario's loot).
#[test]
fn a_creature_blocking_a_band_blocks_each_member() {
    use cantrip_engine::*;
    let path = scripts().join("a-creature-blocking-a-band-triggers-its-blocks-ability-once.cantrip");
    let script = GameScript::load(&path).unwrap();
    let result = run(&script).unwrap_or_else(|f| panic!("\n{}", f.report));
    let game = &result.game;
    let blocks: Vec<(cantrip_engine::ObjectName, cantrip_engine::ObjectName, bool)> = game
        .history()
        .all()
        .iter()
        .filter_map(|r| match r.event {
            Event::Blocks { blocker, attacker, declared } => Some((game.name(blocker), game.name(attacker), declared)),
            _ => None,
        })
        .collect();
    use cantrip_engine::ObjectName::Literal;
    assert_eq!(blocks, vec![(Literal("Smuggler's Copter"), Literal("Benalish Hero"), true), (Literal("Smuggler's Copter"), Literal("Guerrilla Gorilla"), false)]);
}
