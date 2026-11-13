use cantrip_api::*;
use cantrip_mechanics::{effects, targets, triggers};
use cantrip_oracle as oracle;

/// "Destroy target artifact." (CR 701.8a, 608.2b) and "Cycling {R}" (CR 702.29a).
pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::ARTIFACT], effects::destroy_first_target),
    abilities: &[triggers::cycling(mana("{R}"), "Cycling {R} ({R}, Discard this card: Draw a card.)")],
    ..oracle::SKYCRASH
};
