use cantrip_api::*;
use cantrip_mechanics::{effects, targets, triggers::THIS_ENTERS};
use cantrip_oracle as oracle;

/// "Surveil 1." (CR 701.25a).
fn surveil_one(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::surveil(g, item, 1)
}

/// "When this creature enters, choose one —" (CR 603.6a, 700.2b). Each mode uses the shared effect and target rules.
static MODAL: ModalDef = modes(&[
    mode(
        effects::target_creature_gets::<1, 1>,
        "Target creature gets +1/+1 until end of turn.",
    )
    .targets(&[targets::CREATURE]),
    mode(surveil_one, "Surveil 1."),
]);

pub static CARD: CardDef = CardDef {
    abilities: &[modal_triggered(
        THIS_ENTERS,
        &MODAL,
        "When this creature enters, choose one — target creature gets +1/+1 until end of turn; or surveil 1.",
    )],
    ..oracle::FEDERATION_PROBE
};
