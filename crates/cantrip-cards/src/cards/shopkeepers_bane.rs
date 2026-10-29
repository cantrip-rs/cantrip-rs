use cantrip_api::*;
use cantrip_mechanics::{effects, triggers};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(triggers::THIS_ATTACKS, |g, item| effects::gain_life(g, item, 2), "Whenever this creature attacks, you gain 2 life.")],
    ..oracle::SHOPKEEPERS_BANE
};
