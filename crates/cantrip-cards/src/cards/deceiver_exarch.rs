use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_oracle as oracle;

/// "Choose one" as the trigger goes on the stack (CR 603.3c, 700.2a), with the chosen mode's target (CR 603.3d). Mode 0,
/// "Untap target permanent you control" (CR 701.26b), untaps it if it is still legal (`effects::untap_targets`).
static MODAL: ModalDef = modes(&[
    mode(effects::untap_targets, "Untap target permanent you control.").targets(&[targets::PERMANENT_YOU_CONTROL]),
    mode(effects::tap_targets, "Tap target permanent an opponent controls.").targets(&[targets::PERMANENT_AN_OPPONENT_CONTROLS]),
]);

pub static CARD: CardDef = CardDef {
    abilities: &[modal_triggered(THIS_ENTERS, &MODAL, "When this creature enters, choose one \u{2014}\n\u{2022} Untap target permanent you control.\n\u{2022} Tap target permanent an opponent controls.")],
    ..oracle::DECEIVER_EXARCH
};
