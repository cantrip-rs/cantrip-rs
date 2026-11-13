use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    kicker: kicker(mana("{1}{U}")),
    spell: spell(&[targets::NONLAND_PERMANENT], effects::return_first_target_to_hand_then_draw_if_kicked),
    ..oracle::WHOOSH
};
