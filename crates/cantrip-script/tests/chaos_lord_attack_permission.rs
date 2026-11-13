//! The creature that changed controllers can attack that turn if it entered earlier (CR 302.6).
use cantrip_script::{run, GameScript};

#[test]
fn source_can_attack_after_changing_control_during_the_turn() {
    let text = include_str!("../../../scenarios/chaos-lord-overrides-earlier-control.cantrip");
    let script = GameScript::parse(text, "chaos-lord-attack").unwrap();
    let result = run(&script);
    assert!(result.is_ok(), "{}", result.err().map(|failure| failure.report).unwrap_or_default());
}
