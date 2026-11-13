//! Disharmony untaps the attacker before gaining control of it (CR 608.2c).
use cantrip_engine::{Event, PlayerId};
use cantrip_script::{run, GameScript};

#[test]
fn attacker_is_untapped_before_control_changes() {
    let text = include_str!("../../../scenarios/disharmony-steals-attacker-to-block.cantrip");
    let script = GameScript::parse(text, "disharmony-order").unwrap();
    let game = run(&script).unwrap_or_else(|failure| panic!("{}", failure.report)).game;
    let (changed_at, attacker) = game.log.iter().enumerate().find_map(|(i, record)| match record.event {
        Event::ControlChanged { permanent, from: PlayerId(1), controller: PlayerId(0) } => Some((i, permanent)),
        _ => None,
    }).expect("Disharmony gained control");
    assert_eq!(game.name(attacker), "Grizzly Bears");
    let untapped_at = game.log.iter().enumerate().find_map(|(i, record)| match record.event {
        Event::Untapped { permanent } if permanent == attacker => Some(i),
        _ => None,
    }).expect("attacker was untapped");
    assert!(untapped_at < changed_at, "attacker must untap before control changes");
}
