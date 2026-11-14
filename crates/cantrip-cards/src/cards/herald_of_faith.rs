use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[effects::attacks_gain_life::<2>("Whenever this creature attacks, you gain 2 life.")],
    ..oracle::HERALD_OF_FAITH
};
