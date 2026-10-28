use cantrip_api::*;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "Choose one" as the trigger goes on the stack (CR 603.3c, 700.2a), with the chosen mode's target (CR 603.3d). Each mode
/// does its one thing to the target if it is still legal as the ability resolves (CR 608.2b).
static MODAL: ModalDef = modes(&[
    mode(effects::tap_targets, "Tap target nonland permanent.").targets(&[targets::NONLAND_PERMANENT]),
    mode(effects::untap_targets, "Untap target nonland permanent.").targets(&[targets::NONLAND_PERMANENT]),
]);

pub static CARD: CardDef = CardDef {
    abilities: &[modal_triggered(
        THIS_ENTERS,
        &MODAL,
        "When this creature enters, choose one \u{2014}\n\u{2022} Tap target nonland permanent.\n\u{2022} Untap target nonland permanent.",
    )],
    ..oracle::GIANT_SIZED_FLYING_ANT
};
