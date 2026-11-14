use cantrip_api::*;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

static MODES: ModalDef = modes(&[
    mode(effects::two_plus_one_counters_on_target, "Put two +1/+1 counters on target creature you control.")
        .targets(&[targets::CREATURE_YOU_CONTROL]),
    mode(|g, item| effects::gain_life(g, item, 4), "You gain 4 life."),
]);

/// "When this creature enters, choose one —" (CR 700.2, 603.3c): the mode is chosen as the ability is put on the stack, and only the chosen mode's target is chosen.
pub static CARD: CardDef = CardDef {
    abilities: &[modal_triggered(
        THIS_ENTERS,
        &MODES,
        "When this creature enters, choose one — Put two +1/+1 counters on target creature you control; or you gain 4 life.",
    )],
    ..oracle::APOTHECARY_STOMPER
};
