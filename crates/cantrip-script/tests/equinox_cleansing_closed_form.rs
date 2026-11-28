//! Equinox judges Cleansing ("For each land, destroy that land unless any player pays 1 life") by a rule rather than by
//! resolving it every way (`crates/cantrip-cards/src/cards/equinox.rs`, the owner's decisions of 2026-11-27 and 28): it
//! would destroy a land you control only if, with no life paid, it would destroy more of your lands than the life every
//! player could pay. Where the speculative resolution's work bound is not reached, resolving it every way is the reference:
//! every small position - each player's lands, an indestructible land of yours or not, a land of yours with a regeneration
//! shield or not, each player's life - played to the point where Bot2 has cast Cleansing, then the printed Cleansing
//! resolved every way on the hidden copy, must come to what the rule says.

use std::ops::ControlFlow;

use cantrip_engine::*;
use cantrip_script::{GameScript, run};

/// Bot2's Cleansing on the stack, with Bot1 controlling `plains` Plains, if `citadel` Darksteel Citadel, and if `arbor` a
/// Dryad Arbor that Death Ward has given a regeneration shield, Bot2 `islands` Islands, and the players at `life`.
fn position(plains: usize, citadel: bool, arbor: bool, islands: usize, life: (u32, u32)) -> Game {
    let mut mine: Vec<&str> = vec!["Plains"; plains];
    if citadel {
        mine.push("Darksteel Citadel");
    }
    if arbor {
        mine.push("Dryad Arbor");
    }
    let mut source = format!("[Life]\nBot1: {}\nBot2: {}\n\n[Bot1 Deck]\nMox Pearl\nDeath Ward\n", life.0, life.1);
    for land in &mine {
        source += &format!("{land}\n");
    }
    source += "Swamp x20\n\n[Bot2 Deck]\nBlack Lotus\nCleansing\n";
    source += &"Island\n".repeat(islands);
    source += "Swamp x20\n\n[Actions]\nBot1.Hand.Mox Pearl.Cast\n";
    for turn in 1..=9 {
        let (seat, land) = if turn % 2 == 1 { (1, mine.get(turn / 2).copied()) } else { (2, (turn / 2 <= islands).then_some("Island")) };
        if let Some(land) = land {
            source += &format!("[T{turn} Bot{seat}.Main1]\nBot{seat}.Hand.{land}.Play\n");
        }
    }
    source += "[T10 Bot2.Main1]\nBot2.Hand.Black Lotus.Cast\nBot2.Battlefield.Black Lotus.Activate(Color: White)\nBot2.Hand.Cleansing.Cast\n";
    if arbor {
        source += "Bot1.Battlefield.Mox Pearl.Activate\nBot1.Hand.Death Ward.Cast(Dryad Arbor)\nBot1.Pass\nBot2.Pass\n";
    }
    source += &format!("[Handover Bot1.Pass]\n\n[Expect]\nBot1.Battlefield.Count = {}\nBot2.Battlefield.Count = {islands}\nStack.Count = 1\n", mine.len() + 1);
    let script = GameScript::parse(&source, "equinox-cleansing-closed-form").unwrap();
    run(&script).map_err(|f| f.report).unwrap().game
}

/// Equinox's reading: the outcome destroyed a land Bot1 controlled.
fn destroyed_a_land_of(outcome: &Speculated<'_>, you: PlayerId) -> bool {
    outcome.records.iter().any(|r| matches!(r.event,
        Event::ZoneChange { old, reason: ZoneReason::Destroy | ZoneReason::DestroyNoRegen, .. }
            if outcome.game.obj(old).controller == you
                && outcome.game.state().last_known(old).is_some_and(|l| l.types.has(CardType::Land))))
}

#[test]
fn the_rule_for_cleansing_agrees_with_resolving_it_every_way() {
    let you = PlayerId(0);
    let mut checked = 0;
    for plains in 0..=3 {
        for (citadel, arbor) in [(false, false), (true, false), (false, true), (true, true)] {
            for islands in 0..=2 {
                for life in [(1, 1), (1, 2), (2, 1), (2, 2)] {
                    let game = position(plains, citadel, arbor, islands, life);
                    let cleansing = game.state.stack.last().expect("Cleansing is on the stack").object;
                    assert_eq!(game.name(cleansing), "Cleansing");
                    let came = speculate(&game, Target::Object(cleansing),
                        |o| if destroyed_a_land_of(o, you) { ControlFlow::Continue(()) } else { ControlFlow::Break(()) });
                    assert!(matches!(came, Speculation::Decided(()) | Speculation::Undecided), "{came:?}");
                    let every_way = came == Speculation::Undecided;
                    // The rule: Bot1's lands Cleansing would destroy (not the indestructible Citadel, not the regenerating
                    // Arbor) against the life paid.
                    let rule = plains as u32 > life.0 + life.1;
                    assert_eq!(every_way, rule, "{plains} Plains, Citadel {citadel}, shielded Arbor {arbor}, {islands} Islands, life {life:?}");
                    checked += 1;
                }
            }
        }
    }
    assert_eq!(checked, 192);
}
