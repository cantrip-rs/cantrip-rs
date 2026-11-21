use cantrip_api::*;
use cantrip_mechanics::mana;
use cantrip_oracle as oracle;

/// Reach is a core keyword.
pub static CARD: CardDef = CardDef { abilities: &[mana::ONE_FOR_ANY_COLOR_ONCE_EACH_TURN], ..oracle::SCARECROW_GUIDE };
