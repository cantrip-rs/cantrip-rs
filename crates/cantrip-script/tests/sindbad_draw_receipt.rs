//! The original draw's identity is revealed before discard; replacement draws have no follow-up actions (CR 121.6c).
use cantrip_engine::*;
use cantrip_script::{run, GameScript};

fn check(source: &str, reveals: usize, discards: usize) {
    let script = GameScript::parse(source, "sindbad-original-draw").unwrap();
    let result = run(&script).unwrap_or_else(|failure| panic!("{}", failure.report));
    let history = result.game.history();
    let records = history.this_turn();
    let revealed: Vec<_> = records.iter().filter_map(|r| match r.event {
        Event::Revealed { player: PlayerId(0), card } => Some(card),
        _ => None,
    }).collect();
    let discarded: Vec<_> = records.iter().filter_map(|r| match r.event {
        Event::Discarded { player: PlayerId(0), card, .. } => Some(card),
        _ => None,
    }).collect();
    assert_eq!(revealed.len(), reveals);
    assert_eq!(discarded.len(), discards);
    if discards > 0 {
        assert!(revealed.iter().zip(&discarded).all(|(&before, &after)| records.iter().any(|r| {
            matches!(r.event, Event::ZoneChange { old, new, reason: ZoneReason::Discard, .. }
                if old == before && new == after)
        })), "discard the revealed draw's new graveyard incarnation, not another card");
        let reveal_at = records.iter().position(|r| matches!(r.event, Event::Revealed { card, .. } if revealed.contains(&card))).unwrap();
        let discard_at = records.iter().position(|r| matches!(r.event, Event::Discarded { card, .. } if discarded.contains(&card))).unwrap();
        assert!(reveal_at < discard_at);
    }
}

#[test]
fn an_unreplaced_land_is_revealed_and_retained() {
    check(include_str!("../../../scenarios/sindbad-draws-and-keeps-a-land.cantrip"), 1, 0);
}

#[test]
fn an_unreplaced_nonland_is_revealed_then_discarded() {
    check(include_str!("../../../scenarios/sindbad-reveals-and-discards-a-nonland.cantrip"), 1, 1);
}

#[test]
fn archive_replacement_draws_are_neither_revealed_nor_discarded() {
    check(include_str!("../../../scenarios/sindbad-replaced-draw-does-not-reveal-or-discard.cantrip"), 0, 0);
}

#[path = "support/discard_library.rs"]
mod discard_library;

/// The revealed card remains known when a replacement puts it into the library (CR 701.20a, 701.9c).
#[test]
fn a_revealed_discard_remains_known_under_leng() {
    discard_library::play("sindbad-revealed-discard-under-leng", "
[Bot1 Deck]
Probe Leng
Mox Sapphire
Mox Jet
Sindbad
Island x4
Probe Bones
Island x20
[Bot2 Deck]
Island x30
[Actions]
Bot1.Hand.Probe Leng.Cast
Bot1.Pass
Bot2.Pass
Bot1.Hand.Mox Sapphire.Cast
Bot1.Hand.Mox Jet.Cast
Bot1.Battlefield.Mox Sapphire.Activate
Bot1.Battlefield.Mox Jet.Activate
Bot1.Hand.Sindbad.Cast
Bot1.Pass
Bot2.Pass
[T3 Bot1.Main1]
Bot1.Battlefield.Sindbad.Activate
Bot1.Pass
Bot2.Pass
? Stack.Count = 1
Bot1.Pass
Bot2.Pass
[Expect]
Bot1.Life = 23
Bot1.Graveyard.Count = 0
");
}
