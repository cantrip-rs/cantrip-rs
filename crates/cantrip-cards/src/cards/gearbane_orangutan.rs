use cantrip_api::*;
use cantrip_mechanics::{effects, select, targets, triggers};
use cantrip_oracle as oracle;

/// "Sacrifice an artifact. If you do, put two +1/+1 counters on this creature." (CR 701.21a, 608.2c): not optional, so the
/// controller sacrifices one as soon as they have an artifact, choosing when they have several; the counters come only if
/// one was sacrificed, so with none to sacrifice nothing happens.
fn sacrifice_an_artifact_for_two_counters(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if !effects::may_sacrifice(g, item.controller, item.object, select::artifact_cards(select::exactly(1)))?.is_empty() {
        effects::two_plus_one_counters_on_this(g, item)?;
    }
    Ok(())
}

static MODAL: ModalDef = modes(&[
    mode(effects::destroy_first_target, "Destroy up to one target artifact.").targets(&[up_to_one(targets::ARTIFACT.filter)]),
    mode(sacrifice_an_artifact_for_two_counters, "Sacrifice an artifact. If you do, put two +1/+1 counters on this creature."),
]);

pub static CARD: CardDef = CardDef {
    abilities: &[modal_triggered(
        triggers::THIS_ENTERS,
        &MODAL,
        "When this creature enters, choose one — Destroy up to one target artifact; or Sacrifice an artifact. If you do, put two +1/+1 counters on this creature.",
    )],
    ..oracle::GEARBANE_ORANGUTAN
};
