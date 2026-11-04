use cantrip_api::*;
use cantrip_mechanics::{affinity, effects, lands};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    own_cost_changes: &[affinity::for_land_type::<{ lands::sub(Subtype::Town) }>],
    spell: spell(&[], |g, item| effects::draw(g, item, 4)),
    ..oracle::TRAVEL_THE_OVERWORLD
};
