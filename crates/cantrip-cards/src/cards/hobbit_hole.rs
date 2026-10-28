use cantrip_api::*;
use cantrip_mechanics::{lands, typecycling};
use cantrip_oracle as oracle;

/// "{T}, Sacrifice this land: Search your library for a basic land card, put it onto the battlefield tapped, then shuffle." and
/// "Halflingcycling {4}" (CR 702.29e): "{4}, Discard this card: Search your library for a Halfling card, reveal it, put it into your
/// hand, then shuffle", from the hand.
pub static CARD: CardDef = CardDef {
    abilities: &[
        lands::SACRIFICE_TO_FETCH_BASIC_TAPPED,
        typecycling::ability(
            Subtype::Halfling,
            mana("{4}"),
            "Halflingcycling {4} ({4}, Discard this card: Search your library for a Halfling card, reveal it, put it into your hand, then shuffle.)",
        ),
    ],
    ..oracle::HOBBIT_HOLE
};
