use cantrip_api::*;
use cantrip_mechanics::{cost_reduction, effects, targets};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    own_cost_changes: &[cost_reduction::for_each_creature_that_attacked_this_turn],
    spell: spell(&[targets::CREATURE], effects::damage_first_target::<5>),
    ..oracle::WITCHSTALKER_FRENZY
};
