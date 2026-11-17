use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE_CARD_IN_YOUR_GRAVEYARD], effects::return_first_target_to_battlefield_with_two_plus_one_counters),
    ..oracle::EVIL_REAWAKENED
};
