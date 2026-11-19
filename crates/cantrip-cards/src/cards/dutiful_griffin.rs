use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "{2}{W}, Sacrifice two enchantments: Return this card from your graveyard to your hand." It functions only in its owner's
/// graveyard (CR 113.6m), and the two enchantments are sacrificed as part of the cost (CR 118.3, 701.21a).
pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        cost("{2}{W}").and(sacrifice(2, targets::filters::enchantment)),
        effects::return_this_to_hand,
        "{2}{W}, Sacrifice two enchantments: Return this card from your graveyard to your hand.",
    )
    .from(ZoneKind::Graveyard)],
    ..oracle::DUTIFUL_GRIFFIN
};
