use cantrip_api::*;
use cantrip_mechanics::library;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        cost("{3}").and(SACRIFICE_THIS),
        library::search_for_two_basic_lands_onto_the_battlefield_tapped,
        "{3}, Sacrifice this creature: Search your library for up to two basic land cards, put them onto the battlefield tapped, then shuffle.",
    )],
    ..oracle::BURNISHED_HART
};
