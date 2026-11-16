use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_mechanics::expend::EXPEND_4;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        EXPEND_4,
        effects::this_gains_indestructible,
        "Whenever you expend 4, this creature gains indestructible until end of turn. (You expend 4 as you spend your fourth total mana to cast spells during a turn.)",
    )],
    ..oracle::BARK_KNUCKLE_BOXER
};
