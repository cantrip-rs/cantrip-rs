//! Game of Chaos's stake bound (RX-8): flips forced without a seed or the scenario's dice limit (CR 705.3), every one
//! won by the caster, who always flips again while it is offered.
use std::sync::OnceLock;
use cantrip_engine::*;
use cantrip_script::{GameScript, ScriptFailure, ScriptResult, run_on};

fn all_flips_win(_: &dyn Query, asker: &Asker, player: PlayerId) -> bool { player == asker.controller }
static FIXER: CardDef = CardDef {
    name: "Forced coin probe", oracle: true, has_cost: true, cost: ManaCost::ZERO,
    types: TypeSet::of(&[CardType::Artifact]), coin_flip_rules: &[all_flips_win],
    ..CardDef::BLANK
};

fn table() -> &'static CardTable {
    static TABLE: OnceLock<CardTable> = OnceLock::new();
    TABLE.get_or_init(|| CardTable::new(cantrip_cards::all().into_iter().chain([&FIXER])))
}

/// The game from `life` (Bot1's, Bot2's) after `accepts` flips again, then whatever `tail` says.
fn play(life: (i32, i32), accepts: usize, tail: &str) -> Result<ScriptResult, ScriptFailure> {
    let mut text = format!("[Life]\nBot1: {}\nBot2: {}\n", life.0, life.1);
    text.push_str(r#"
[Bot1 Deck]
Forced coin probe
Game of Chaos
Black Lotus
Mountain x20
[Bot2 Deck]
Island x20
[Actions]
Bot1.Hand.Forced coin probe.Cast
Bot1.Hand.Black Lotus.Cast
Bot1.Battlefield.Black Lotus.Activate(Color: Red)
Bot1.Hand.Game of Chaos.Cast(Bot2)
Bot1.Pass
Bot2.Pass
"#);
    for _ in 0..accepts { text.push_str("Bot1.Accept\n"); }
    text.push_str(tail);
    run_on(&GameScript::parse(&text, "game-of-chaos-large-stake").unwrap(), table())
}

fn flips(result: &ScriptResult) -> usize {
    result.game.log.iter().filter(|r| matches!(r.event, Event::CoinFlipped { .. })).count()
}

/// From 1 life each, after flip n the caster is at 2^n and the opponent at 2 - 2^n; the next stake 2^n is offered while
/// 2^(n+1) <= 2^24, so after flips 1 to 23. The 24th flip takes the caster exactly to the bound and ends the spell
/// without a question, both totals exact, with no wrap.
#[test]
fn the_flip_again_offer_stops_at_the_stake_bound() {
    let result = play((1, 1), 23, "[Expect]\nBot1.Life = 16777216\nBot2.Life = -16777214\nResult = Bot1 won\n")
        .unwrap_or_else(|f| panic!("{}", f.report));
    assert_eq!(flips(&result), 24);
}

/// A 24th "flip again" is never there to accept: the game ends with that line unmatched.
#[test]
fn no_offer_is_made_past_the_bound() {
    let Err(failure) = play((1, 1), 24, "[Expect]\nBot1.Life = 33554432\n") else { panic!("a 25th flip was offered past the bound") };
    assert!(failure.report.contains("the game ended before line 42: Bot1.Accept"), "{}", failure.report);
}

/// The bound reads the next, doubled stake, not the one just played: from 13,631,489 (2^24 - 3 * 2^20 + 1), the caster is
/// at 13,631,488 + 2^n after flip n, and offered the stake 2^n while 13,631,488 + 2^(n+1) <= 2^24, so after flips 1 to 20.
#[test]
fn the_bound_reads_the_doubled_stake() {
    let result = play((13_631_489, 20), 20, "[Expect]\nBot1.Life = 15728640\nBot2.Life = -2097131\n")
        .unwrap_or_else(|f| panic!("{}", f.report));
    assert_eq!(flips(&result), 21);
    let Err(failure) = play((13_631_489, 20), 21, "") else { panic!("a 22nd flip was offered past the bound") };
    assert!(failure.report.contains("the game ended before line 39: Bot1.Accept"), "{}", failure.report);
}

/// The bound reads both players' totals, the target's as well as the caster's: with Bloodletter of Aclazotz the target
/// loses twice each stake, so its total is the one that reaches the bound first. From 1 and 20, after flip n the caster
/// is at 2^n and the target at 22 - 2^(n+1); the stake 2^n is offered while 2^(n+1) - 22 + 2^n <= 2^24 for the target
/// (after flips 1 to 22), where the caster's own total alone would allow one more (2^(n+1) <= 2^24, flips 1 to 23).
#[test]
fn the_bound_reads_the_targets_total_too() {
    let run = |accepts: usize, tail: &str| {
        let mut text = String::from(r#"[Life]
Bot1: 1
Bot2: 20
[Bot1 Deck]
Forced coin probe
Bloodletter of Aclazotz
Game of Chaos
Black Lotus x3
Mountain x20
[Bot2 Deck]
Island x20
[Actions]
Bot1.Hand.Forced coin probe.Cast
Bot1.Hand.Black Lotus#1.Cast
Bot1.Hand.Black Lotus#1.Cast
Bot1.Hand.Black Lotus.Cast
Bot1.Battlefield.Black Lotus#1.Activate(Color: Black)
Bot1.Battlefield.Black Lotus#1.Activate(Color: Red)
Bot1.Hand.Bloodletter of Aclazotz.Cast
Bot1.Battlefield.Black Lotus.Activate(Color: Red)
Bot1.Hand.Game of Chaos.Cast(Bot2)
Bot1.Pass
Bot2.Pass
"#);
        for _ in 0..accepts { text.push_str("Bot1.Accept\n"); }
        text.push_str(tail);
        run_on(&GameScript::parse(&text, "game-of-chaos-target-bound").unwrap(), table())
    };
    let result = run(22, "[Expect]\nBot1.Life = 8388608\nBot2.Life = -16777194\n").unwrap_or_else(|f| panic!("{}", f.report));
    assert_eq!(flips(&result), 23);
    let Err(failure) = run(23, "") else { panic!("a 24th flip was offered past the target's bound") };
    assert!(failure.report.contains("the game ended before line") && failure.report.contains("Bot1.Accept"), "{}", failure.report);
}
