use cantrip_api::*;
use cantrip_mechanics::{effects, targets, triggers};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE], effects::target_gains_double_strike),
    abilities: &[triggers::cycling(mana("{2}"), "Cycling {2} ({2}, Discard this card: Draw a card.)")],
    ..oracle::RAKING_CLAWS
};
