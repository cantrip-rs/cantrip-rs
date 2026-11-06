//! The Aura has no enchanted creature between Go Ninja Go's two modes; it must not prevent the second mode's damage.
use cantrip_script::{GameScript, run};

#[test]
fn unattached_aura_does_not_prevent_damage_during_resolution() {
    let text = include_str!("../../../scenarios/gaseous-form-does-not-prevent-damage-after-host-blinks.cantrip");
    let script = GameScript::parse(text, "gaseous-form-unattached").expect("scenario parses");
    let result = run(&script);
    assert!(result.is_ok(), "{}", result.err().map(|failure| failure.report).unwrap_or_default());
}
