use cantrip_api::*;
use cantrip_mechanics::{effects, targets, triggers::THIS_ENTERS};
use cantrip_oracle as oracle;

/// "Choose one —" (CR 700.2): one mode, its own target if it has one (CR 700.2c), in printed order (CR 700.2d).
static MODAL: ModalDef = modes(&[
    mode(effects::return_first_target_to_hand, "Return target artifact card from your graveyard to your hand.")
        .targets(&[targets::ARTIFACT_CARD_IN_YOUR_GRAVEYARD]),
    mode(|g, item| effects::scry(g, item, 3), "Scry 3."),
]);

pub static CARD: CardDef = CardDef {
    abilities: &[modal_triggered(
        THIS_ENTERS,
        &MODAL,
        "When this creature enters, choose one —\n\u{2022} Return target artifact card from your graveyard to your hand.\n\u{2022} Scry 3.",
    )],
    ..oracle::OLTEC_ARCHAEOLOGISTS
};
