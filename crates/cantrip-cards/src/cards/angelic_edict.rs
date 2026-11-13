use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    spell: spell(&[target(targets::creature_or_enchantment)], effects::exile_first_target),
    ..oracle::ANGELIC_EDICT
};
