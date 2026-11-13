use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "Target creature gets -1/-0 until end of turn. Draw a card." (CR 611.2a, 121.1). If the creature is no longer a legal target as the
/// spell resolves, the spell does not resolve and no card is drawn (CR 608.2b).
pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE], effects::target_creature_gets_then_draw_a_card::<-1, 0>),
    ..oracle::FLEETING_DISTRACTION
};
