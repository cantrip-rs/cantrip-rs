use cantrip_script::{run, GameScript};
use std::path::Path;

#[test]
fn sacrifice_tracks_its_own_creature_through_a_casualty_copy() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../scenarios/sacrifice-distinguishes-casualty-cost.cantrip");
    let script = GameScript::load(&path).unwrap();
    run(&script).unwrap_or_else(|failure| panic!("{}", failure.report));
}

#[test]
fn sacrifice_reads_the_copied_creatures_last_known_mana_value() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../scenarios/sacrifice-uses-copied-creatures-last-known-mana-value.cantrip");
    let script = GameScript::load(&path).unwrap();
    run(&script).unwrap_or_else(|failure| panic!("{}", failure.report));
}
