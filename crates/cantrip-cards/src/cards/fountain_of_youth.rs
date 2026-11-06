use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[ability(cost("{2}").and(TAP), effects::gain_one_life, "{2}, {T}: You gain 1 life.")],
    ..oracle::FOUNTAIN_OF_YOUTH
};
