use cantrip_api::*;
use cantrip_mechanics::{effects, targets, triggers::cycling};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE_CARD_WITH_MANA_VALUE_3_OR_LESS_IN_YOUR_GRAVEYARD], effects::return_target_card_to_battlefield),
    abilities: &[cycling(mana("{2}"), "Cycling {2} ({2}, Discard this card: Draw a card.)")],
    ..oracle::UNEARTH
};
