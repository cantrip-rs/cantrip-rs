use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    spell: spell(&[], effects::draw_three_then_discard),
    ..oracle::ANCESTRAL_REMINISCENCE
};
