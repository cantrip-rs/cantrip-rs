use cantrip_api::*;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_mechanics::{effects, sneak};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(THIS_ENTERS, |g, item| effects::gain_life(g, item, 3), "When this creature enters, you gain 3 life.")],
    ways: &[sneak::way(mana("{3}{W/B}"), "Sneak {3}{W/B}")],
    ..oracle::FOOT_NINJAS
};
