use cantrip_api::*;
use cantrip_mechanics::{effects, targets, triggers};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(triggers::ENCHANTED_PLAYER_ATTACKED, effects::plus_one_counter_on_that_permanent, "Whenever a creature attacks enchanted player, put a +1/+1 counter on it.")],
    enchant: Some(targets::PLAYER),
    ..oracle::CURSE_OF_PREDATION
};
