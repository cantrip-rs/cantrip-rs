use cantrip_api::*;
use cantrip_mechanics::mana;
use cantrip_oracle as oracle;

/// "Changeling" comes from the printed keywords (CR 702.73a).
pub static CARD: CardDef = CardDef { abilities: &[mana::ONE_FOR_ANY_COLOR_ONCE_EACH_TURN], ..oracle::THREE_TREE_MASCOT };
