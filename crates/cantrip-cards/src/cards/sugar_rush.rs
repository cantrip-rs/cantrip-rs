use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "Target creature gets +3/+0 until end of turn. Draw a card." (CR 611.2c, 121.1): with the target gone nothing happens, the
/// draw included (CR 608.2b).
pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE], effects::target_creature_gets_then_draw_a_card::<3, 0>),
    ..oracle::SUGAR_RUSH
};
