use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    spell: spell(&[], effects::draw_then_discard::<3, 1>),
    ..oracle::ANCESTRAL_REMINISCENCE
};
