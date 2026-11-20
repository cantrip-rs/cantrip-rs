use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[ability(TAP, effects::draw_then_discard::<2, 3>, "{T}: Draw two cards, then discard three cards.")],
    ..oracle::BAZAAR_OF_BAGHDAD
};
