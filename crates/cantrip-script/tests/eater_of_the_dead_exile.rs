use cantrip_engine::{Event, Query, ZoneKind, ZoneReason};
use cantrip_script::{run, GameScript};
use std::path::Path;

#[test]
fn eater_records_an_exile_action() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../scenarios/eater-of-the-dead-leaves-tapped-before-resolution.cantrip");
    let script = GameScript::load(&path).unwrap();
    let result = run(&script).unwrap_or_else(|failure| panic!("{}", failure.report));
    let game = &result.game;
    let reasons: Vec<_> = game.history().all().iter().filter_map(|record| match record.event {
        Event::ZoneChange { old, from, to, reason, .. }
            if game.name(old) == "Memnite" && from.is(ZoneKind::Graveyard) && to.is(ZoneKind::Exile) => Some(reason),
        _ => None,
    }).collect();
    assert_eq!(reasons, vec![ZoneReason::Exile]);
}
