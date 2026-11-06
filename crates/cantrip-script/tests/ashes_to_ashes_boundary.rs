//! The exile event and the subsequent damage of Ashes to Ashes (CR 603.8, 608.2c).
use cantrip_engine::*;
use cantrip_script::{GameScript, run_on};

#[test]
fn exile_and_damage_precede_jihad_sacrifice() {
    let text = include_str!("../../../scenarios/ashes-to-ashes-last-chosen-color-queues-jihad.cantrip");
    let script = GameScript::parse(text, "ashes-boundary").unwrap();
    let g = run_on(&script, cantrip_engine::cards()).unwrap().game;
    let named = |id: ObjectId, name: &str| g.cards.get(g.state.obj(id).card).name == name;
    let exile = g.log.iter().position(|r| matches!(r.event, Event::ZoneChange { old, from, to, .. } if named(old, "Llanowar Elves") && from.is(ZoneKind::Battlefield) && to.is(ZoneKind::Exile))).unwrap();
    let damage = g.log.iter().position(|r| matches!(r.event, Event::Damage { source, target: Target::Player(PlayerId(0)), amount: 5, .. } if named(source, "Ashes to Ashes"))).unwrap();
    let sacrifice = g.log.iter().position(|r| matches!(r.event, Event::ZoneChange { old, from, to, .. } if named(old, "Jihad") && from.is(ZoneKind::Battlefield) && to.is(ZoneKind::Graveyard))).unwrap();
    assert!(exile < damage && damage < sacrifice, "exile={exile}, damage={damage}, sacrifice={sacrifice}");
}
