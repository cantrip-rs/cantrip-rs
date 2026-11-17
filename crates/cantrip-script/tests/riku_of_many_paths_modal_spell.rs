//! Riku of Many Paths: "Whenever you cast a modal spell" (CR 700.2) reads the modes chosen for that spell, not those of the
//! last modal spell cast. Every modes question Riku's trigger should ask is answered by a line of the scenario, so a
//! defaulted one means it triggered for the Ornithopter (cast after Kolaghan's Command) or Holy Day (after Bot2's Boros Charm).
use cantrip_script::{GameScript, run};

#[test]
fn a_non_modal_spell_asks_for_no_modes() {
    let source = include_str!("../../../scenarios/riku-of-many-paths-chooses-up-to-as-many-modes-as-were-chosen-for-your-modal-spell-and-ignores-other-spells.cantrip");
    let script = GameScript::parse(source, "riku-modal-spell").unwrap();
    let result = run(&script).map_err(|failure| failure.report).unwrap();
    assert!(result.trace.iter().all(|line| !(line.contains("(default)") && line.contains("ChooseModes"))),
        "Riku triggered for a spell with no modes chosen: {:?}", result.trace);
}
