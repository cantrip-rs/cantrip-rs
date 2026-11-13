//! Check the amount and identity of damage in Axelrod's death trigger.
use cantrip_script::{GameScript, run};

fn check(text: &str) {
    let script = GameScript::parse(text, "axelrod-history").unwrap();
    let result = run(&script);
    assert!(result.is_ok(), "{}", result.err().map(|e| e.report).unwrap_or_default());
}

#[test]
fn one_damage_to_a_creature_counts() {
    check(include_str!("../../../scenarios/axelrod-gunnarson-one-damage.cantrip"));
}

#[test]
fn damage_to_a_player_and_damage_from_another_source_do_not_match() {
    check(include_str!("../../../scenarios/axelrod-gunnarson-unrelated-damage.cantrip"));
}
