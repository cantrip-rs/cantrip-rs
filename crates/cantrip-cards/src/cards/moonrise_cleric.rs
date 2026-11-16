use cantrip_api::*;
use cantrip_mechanics::{effects, triggers::THIS_ATTACKS};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(THIS_ATTACKS, effects::gain_one_life, "Whenever this creature attacks, you gain 1 life.")],
    ..oracle::MOONRISE_CLERIC
};
