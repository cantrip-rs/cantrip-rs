//! Remove Enchantments' empty first group and a return replaced by unearth (CR 608.2c, 702.84a).
use cantrip_engine::*;
use cantrip_script::{GameScript, run_on};

fn play(text: &str) -> Game {
    let script = GameScript::parse(text, "remove-enchantments-boundary").unwrap();
    run_on(&script, cantrip_engine::cards()).unwrap().game
}
fn named(g: &Game, id: ObjectId, name: &str) -> bool { g.cards.get(g.state.obj(id).card).name == name }

#[test]
fn no_owned_matching_permanents_still_destroys_the_other_aura() {
    let g = play(include_str!("../../../scenarios/remove-enchantments-destroys-opponents-enchantment-given-to-caster.cantrip"));
    let changes: Vec<_> = g.log.iter().filter(|r| matches!(r.cause, Cause::Spell(spell) if named(&g, spell, "Remove Enchantments"))).filter_map(|r| match r.event {
        Event::ZoneChange { old, from, to, .. } if from.is(ZoneKind::Battlefield) => Some((g.cards.get(g.state.obj(old).card).name, to.kind())),
        _ => None,
    }).collect();
    assert_eq!(changes, [("Escape Velocity", ZoneKind::Graveyard)]);
}

#[test]
fn unearthed_enchantment_leaves_for_exile_in_place_of_the_attempted_return() {
    let g = play(include_str!("../../../scenarios/remove-enchantments-unearthed-enchantment-is-exiled.cantrip"));
    let moves: Vec<_> = g.log.iter().filter(|r| matches!(r.cause, Cause::Spell(spell) if named(&g, spell, "Remove Enchantments"))).filter_map(|r| match r.event {
        Event::ZoneChange { old, new, from, to, .. } if named(&g, old, "Dregscape Zombie") && from.is(ZoneKind::Battlefield) => Some((old, new, to.kind())),
        _ => None,
    }).collect();
    assert_eq!(moves.len(), 1);
    assert_ne!(moves[0].0, moves[0].1);
    assert_eq!(moves[0].2, ZoneKind::Exile);
}
