use cantrip_api::*;
use cantrip_mechanics::replacements::REVEAL_AND_SHUFFLE_INTO_LIBRARY_INSTEAD_OF_GRAVEYARD;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    // "If Darksteel Colossus would be put into a graveyard from anywhere, reveal Darksteel Colossus and shuffle it into its
    // owner's library instead." (CR 614.1a, 113.6b.) Trample and indestructible come from the printed keywords.
    replacements: &[REVEAL_AND_SHUFFLE_INTO_LIBRARY_INSTEAD_OF_GRAVEYARD],
    ..oracle::DARKSTEEL_COLOSSUS
};
