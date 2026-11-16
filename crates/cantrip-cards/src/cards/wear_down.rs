use cantrip_api::*;
use cantrip_mechanics::{effects, gift, targets};
use cantrip_oracle as oracle;

/// "Destroy target artifact or enchantment. If the gift was promised, instead destroy two target artifacts and/or
/// enchantments." The promise is announced before the targets are chosen (CR 601.2b-c, 702.174m), so the one slot asks for
/// two when it was (`if_promised`); the gift itself is given before this runs (CR 702.174j). Each target still legal is
/// destroyed together (CR 701.8a, 608.2b).
pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::ARTIFACT_OR_ENCHANTMENT.if_promised(2, 2)], effects::destroy_each_target),
    gift: Some(gift::CARD),
    ..oracle::WEAR_DOWN
};
