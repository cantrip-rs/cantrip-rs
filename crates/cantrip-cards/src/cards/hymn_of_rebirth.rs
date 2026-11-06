use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE_CARD_IN_A_GRAVEYARD], effects::return_first_target_to_battlefield),
    ..oracle::HYMN_OF_REBIRTH
};
