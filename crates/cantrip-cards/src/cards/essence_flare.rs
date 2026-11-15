use cantrip_api::*;
use cantrip_mechanics::{attach, statics::plus_pt, targets, triggers::ENCHANTED_PERMANENTS_CONTROLLERS_UPKEEP_HOST};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        ENCHANTED_PERMANENTS_CONTROLLERS_UPKEEP_HOST,
        |g, item| attach::put_counter_on_that_creature(g, item, CounterKind::Plus0Minus1),
        "At the beginning of the upkeep of enchanted creature's controller, put a -0/-1 counter on that creature.",
    )],
    statics: &[static_ability(Layer::PtModify, attach::attached_object, plus_pt::<2, 0>, "Enchanted creature gets +2/+0.")],
    enchant: Some(targets::CREATURE),
    ..oracle::ESSENCE_FLARE
};
