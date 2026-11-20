use cantrip_api::*;
use cantrip_mechanics::{cost_reduction, effects};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    spell: spell(&[], |g, item| effects::draw(g, item, 3)),
    own_cost_changes: &[cost_reduction::for_each_creature_that_attacked_this_turn],
    ..oracle::ROWDY_RESEARCH
};
