use cantrip_api::*;
use cantrip_mechanics::crew::crew;
use cantrip_mechanics::lands::may_search_basic_land_to_hand;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_oracle as oracle;

/// "You may search your library for a basic land card, reveal it, put it into your hand, then shuffle" (CR 701.23).
fn search(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    may_search_basic_land_to_hand(g, item.controller, item.object)
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            THIS_ENTERS,
            search,
            "When this Vehicle enters, you may search your library for a basic land card, reveal it, put it into your hand, then shuffle.",
        ),
        crew(2, "Crew 2 (Tap any number of creatures you control with total power 2 or more: This Vehicle becomes an artifact creature until end of turn.)"),
    ],
    ..oracle::STRIXHAVEN_SKYCOACH
};
