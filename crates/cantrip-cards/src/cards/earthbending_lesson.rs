use cantrip_api::*;
use cantrip_mechanics::earthbend;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    spell: spell(&[earthbend::TARGET], earthbend::effect::<4>),
    delayed: &[earthbend::RETURN],
    ..oracle::EARTHBENDING_LESSON
};
