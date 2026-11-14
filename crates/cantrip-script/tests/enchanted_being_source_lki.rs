//! Enchanted Being's prevention asks about the damage source as it last existed (CR 113.7a, 615.1).
use cantrip_engine::*;
use cantrip_script::{GameScript, run};

fn applies(scenario: &str, combat: bool) -> bool {
    let script = GameScript::parse(scenario, "enchanted-being-source-lki").unwrap();
    let game = run(&script).map_err(|failure| failure.report).unwrap().game;
    let wall = game.state.battlefield().iter().copied().find(|&id|
        game.cards.get(game.state.obj(id).card).name == "Wall of Putrid Flesh").unwrap();
    let firebrand = game.history().this_turn().iter().find_map(|r| match r.event {
        Event::ZoneChange { old, reason: ZoneReason::Sacrifice, .. }
            if game.cards.get(game.state.obj(old).card).name == "Fanatical Firebrand" => Some(old),
        _ => None,
    }).unwrap();
    let being = cantrip_cards::all().into_iter().find(|c| c.name == "Enchanted Being").unwrap();
    let event = DamageEvent { source: firebrand, target: Target::Object(wall), amount: 1, combat, heal_marked: false, shielded: None };
    (being.preventions[0].applies)(&game, &Asker::new(wall, PlayerId(0)), &event)
}

#[test]
fn enchanted_being_reads_a_departed_damage_source_and_requires_combat() {
    let enchanted = include_str!("../../../scenarios/wall-of-putrid-flesh-enchanted-sacrificed-source.cantrip");
    let plain = include_str!("../../../scenarios/wall-of-putrid-flesh-unenchanted-sacrificed-source.cantrip");
    assert!(applies(enchanted, true));
    assert!(!applies(enchanted, false));
    assert!(!applies(plain, true));
}
