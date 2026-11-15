use cantrip_api::*;
use cantrip_mechanics::{attach, targets};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    // "Enchant Wall" (CR 702.5a, 303.4a).
    enchant: Some(targets::WALL),
    // "Enchanted Wall can attack as though it didn't have defender." (CR 702.3b): the permanent it is attached to.
    attacks_despite_defender: &[attach::attached_to_it],
    ..oracle::ANIMATE_WALL
};
