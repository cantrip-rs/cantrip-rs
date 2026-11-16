use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_mechanics::expend::EXPEND_4;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        EXPEND_4,
        // "This creature deals 2 damage to each opponent" (CR 120.3), from this creature as it last existed if it has
        // left the battlefield (CR 113.7a).
        effects::damage_each_opponent_n::<2>,
        "Whenever you expend 4, this creature deals 2 damage to each opponent. (You expend 4 as you spend your fourth total mana to cast spells during a turn.)",
    )],
    ..oracle::TEAPOT_SLINGER
};
