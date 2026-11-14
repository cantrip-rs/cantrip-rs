//! Sword's damage happens before the sacrificed cards are exiled (CR 608.2c).
use cantrip_engine::*;
use cantrip_script::{GameScript, run};

#[test]
fn damage_precedes_exiling_the_sacrificed_cards() {
    let text = include_str!("../../../scenarios/sword-of-the-ages-sacrifices-creatures-for-damage.cantrip");
    let script = GameScript::parse(text, "sword-order").unwrap();
    let game = run(&script).unwrap().game;
    let named = |id: ObjectId, name: &str| game.cards.get(game.state.obj(id).card).name == name;
    let damage = game.log.iter().position(|r| matches!(r.event,
        Event::Damage { source, amount: 1, .. } if named(source, "Sword of the Ages"))).expect("Sword deals one damage");
    for name in ["Sword of the Ages", "Memnite", "Ornithopter"] {
        let exile = game.log.iter().position(|r| matches!(r.event,
            Event::ZoneChange { old, from, to, .. } if named(old, name) && from.is(ZoneKind::Graveyard) && to.is(ZoneKind::Exile)))
            .unwrap_or_else(|| panic!("{name} was exiled"));
        assert!(damage < exile, "{name} was exiled before the damage");
    }
}
