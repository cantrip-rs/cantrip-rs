use cantrip_api::*;
use cantrip_mechanics::{damage_each_target, effects, targets};
use cantrip_oracle as oracle;

static MODAL: ModalDef = modes(&[
    mode(effects::surveil_then_draw::<2>, "Surveil 2, then draw a card."),
    mode(|g, item| { damage_each_target(g, item, 1); Ok(()) }, "Prismari Charm deals 1 damage to each of one or two targets.").targets(&[targets::ANY.between(1, 2)]),
    mode(effects::return_first_target_to_hand, "Return target nonland permanent to its owner's hand.")
        .targets(&[targets::NONLAND_PERMANENT]),
]);

pub static CARD: CardDef = CardDef { spell: Some(SpellDef::MODAL), modal: Some(&MODAL), ..oracle::PRISMARI_CHARM };
