use cantrip_api::*;
use cantrip_mechanics::{combat, effects, lands, triggers};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    block_rules: &[combat::cant_be_blocked_by_subtype::<{ lands::sub(Subtype::Human) }>],
    abilities: &[triggered(
        triggers::DEALS_COMBAT_DAMAGE_TO_PLAYER_SUBJECT,
        effects::plus_one_counter_on_this,
        "Whenever this creature deals combat damage to a player, put a +1/+1 counter on it.",
    )],
    ..oracle::STROMKIRK_NOBLE
};
