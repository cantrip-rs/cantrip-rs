use cantrip_api::*;
use cantrip_mechanics::prepare::target_becomes_prepared;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

/// "Target creature becomes unprepared" (CR 722.3b): nothing if it is not prepared, which the verb checks.
fn unprepare(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(id) = targets::object(g, item, 0) else { return Ok(()) };
    g.become_unprepared(id);
    Ok(())
}

/// "Choose up to one" as the trigger goes on the stack (CR 603.3c, 700.2a, 700.2b): with none chosen it is removed from the
/// stack; a chosen mode brings its target (CR 603.3d).
static MODAL: ModalDef = modes(&[
    mode(target_becomes_prepared, "Target creature becomes prepared.").targets(&[targets::CREATURE]),
    mode(unprepare, "Target creature becomes unprepared.").targets(&[targets::CREATURE]),
])
.choose(0, 1);

pub static CARD: CardDef = CardDef {
    abilities: &[modal_triggered(
        THIS_ENTERS,
        &MODAL,
        "When this creature enters, choose up to one \u{2014}\n\u{2022} Target creature becomes prepared.\n\u{2022} Target creature becomes unprepared.",
    )],
    ..oracle::BIBLIOPLEX_TOMEKEEPER
};
