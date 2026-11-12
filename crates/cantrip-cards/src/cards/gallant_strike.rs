use cantrip_api::*;
use cantrip_mechanics::{effects, targets, triggers};
use cantrip_oracle as oracle;

/// "Destroy target creature with toughness 4 or greater." (CR 701.8, 115.1): the toughness is checked as the spell is cast
/// and again as it resolves (CR 608.2b). "Cycling {2}" (CR 702.29a) is the shared mechanic.
pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE_WITH_TOUGHNESS_4_OR_GREATER], effects::destroy_first_target),
    abilities: &[triggers::cycling(mana("{2}"), "Cycling {2} ({2}, Discard this card: Draw a card.)")],
    ..oracle::GALLANT_STRIKE
};
