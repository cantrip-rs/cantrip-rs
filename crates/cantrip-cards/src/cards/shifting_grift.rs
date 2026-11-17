use cantrip_api::*;
use cantrip_mechanics::{exchange, spree, targets};
use cantrip_oracle as oracle;

/// Each mode exchanges control of two different targets of its own type (CR 701.12a-b, 601.2c); the exchange is made only if
/// both are still legal as it resolves, and two permanents one player controls exchange nothing (the rulings).
const MODES: &[ModeDef] = &[
    mode(exchange::control_of_two_targets, "+ {2} — Exchange control of two target creatures.")
        .targets(&[targets::CREATURE.between(2, 2)])
        .cost(mana("{2}")),
    mode(exchange::control_of_two_targets, "+ {1} — Exchange control of two target artifacts.")
        .targets(&[targets::ARTIFACT.between(2, 2)])
        .cost(mana("{1}")),
    mode(exchange::control_of_two_targets, "+ {1} — Exchange control of two target enchantments.")
        .targets(&[targets::ENCHANTMENT.between(2, 2)])
        .cost(mana("{1}")),
];

static MODAL: ModalDef = spree::modal(MODES);

pub static CARD: CardDef = CardDef { spell: Some(SpellDef::MODAL), modal: Some(&MODAL), ..oracle::SHIFTING_GRIFT };
