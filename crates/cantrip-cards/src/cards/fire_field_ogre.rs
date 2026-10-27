use cantrip_api::*;
use cantrip_mechanics::unearth;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        unearth::ability(mana("{U}{B}{R}"), "Unearth {U}{B}{R}"),
    ],
    delayed: &[unearth::EXILE],
    ..oracle::FIRE_FIELD_OGRE
};
