use cantrip_api::*;
use cantrip_mechanics::lands::may_search_basic_land_to_top;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_oracle as oracle;

/// "When this creature enters, you may search your library for a basic land card, reveal it, then shuffle and put that card on
/// top" is `lands::may_search_basic_land_to_top`, the same search Fountainport Bell makes when it enters.
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        THIS_ENTERS,
        |g, item| may_search_basic_land_to_top(g, item.controller, item.object),
        "When this creature enters, you may search your library for a basic land card, reveal it, then shuffle and put that card on top.",
    )],
    ..oracle::CAMPUS_GUIDE
};
