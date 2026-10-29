use cantrip_api::*;
use cantrip_mechanics::{effects, triggers};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    enters: Some(effects::enters_with_x_counters),
    abilities: &[triggered(triggers::THIS_ENTERS, |g, item| effects::gain_life(g, item, 2), "When this creature enters, you gain 2 life.")],
    ..oracle::PTERAFRACTYL
};
