use cantrip_api::*;
use cantrip_mechanics::{effects, targets, tokens, triggers};
use cantrip_oracle as oracle;

static MODAL: ModalDef = modes(&[
    mode(|g, item| tokens::create_food(g, item.controller), "Create a Food token."),
    mode(effects::plus_one_counter_on_target, "Put a +1/+1 counter on target creature you control.").targets(&[targets::CREATURE_YOU_CONTROL]),
    mode(effects::scry_two, "Scry 2."),
]);

pub static CARD: CardDef = CardDef {
    abilities: &[modal_triggered(
        triggers::THIS_LEAVES,
        &MODAL,
        "When Momo leaves the battlefield, choose one \u{2014} Create a Food token; or put a +1/+1 counter on target creature you control; or scry 2.",
    )],
    ..oracle::MOMO_PLAYFUL_PET
};

pub static EXTRA: &[&CardDef] = &[&tokens::FOOD];
