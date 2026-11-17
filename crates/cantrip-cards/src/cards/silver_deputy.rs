use cantrip_api::*;
use cantrip_mechanics::library::{ask_may_search, find_to_top};
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_mechanics::{effects, select, targets};
use cantrip_oracle as oracle;

const ASK: Question = Question::new(
    "SearchLibrary",
    "search your library for a basic land card or a Desert card?",
    "Reveal it, then shuffle and put that card on top of your library",
    "Search",
    "Don't search",
);

/// "When this creature enters, you may search your library for a basic land card or a Desert card, reveal it, then shuffle and
/// put it on top" (CR 701.23): asked first, and a declined search shuffles nothing.
fn search_for_a_basic_land_or_desert_to_top(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if ask_may_search(g, item.controller, item.object, &ASK)? {
        find_to_top(g, item.controller, item.object, select::BASIC_LAND_OR_DESERT_CARD, true)?;
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            THIS_ENTERS,
            search_for_a_basic_land_or_desert_to_top,
            "When this creature enters, you may search your library for a basic land card or a Desert card, reveal it, then shuffle and put it on top.",
        ),
        ability(TAP, effects::target_creature_gets::<1, 0>, "{T}: Target creature you control gets +1/+0 until end of turn. Activate only as a sorcery.")
            .targets(&[targets::CREATURE_YOU_CONTROL])
            .sorcery_speed(),
    ],
    ..oracle::SILVER_DEPUTY
};
