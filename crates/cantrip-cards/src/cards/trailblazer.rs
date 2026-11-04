use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE], effects::target_creature_cant_be_blocked_this_turn),
    ..oracle::TRAILBLAZER
};
