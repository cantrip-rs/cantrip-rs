use cantrip_api::*;
use cantrip_mechanics::effects::first_target_gains_flying;
use cantrip_mechanics::{airbend_targets, targets, triggers};
use cantrip_oracle as oracle;

static MODAL: ModalDef = modes(&[
    mode(first_target_gains_flying, "Target creature you control gains flying until end of turn.")
        .targets(&[targets::CREATURE_YOU_CONTROL]),
    mode(airbend_targets, "Airbend another target nonland permanent you control.")
        .targets(&[target(targets::other_nonland_permanent_you_control_filter)]),
]);

pub static CARD: CardDef = CardDef {
    abilities: &[modal_triggered(
        triggers::THIS_ENTERS_OR_ATTACKS,
        &MODAL,
        "Whenever Appa enters or attacks, choose one \u{2014} Target creature you control gains flying until end of turn; or airbend another target nonland permanent you control.",
    )],
    ..oracle::APPA_LOYAL_SKY_BISON
};
