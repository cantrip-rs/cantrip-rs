use cantrip_api::*;
use cantrip_mechanics::{beam_me_up, library, select};
use cantrip_oracle as oracle;

/// "A basic land card or Planet card": the subtype word is read through this spell's text (CR 612.2).
fn basic_land_or_planet(q: &dyn Query, choice: &cantrip_api::select::Choice, _: u32) -> cantrip_api::select::Verdict {
    let planet = q.words(choice.source).subtype(Subtype::Planet);
    cantrip_api::select::Verdict::each(choice, |card| {
        cantrip_mechanics::lands::is_basic_land_card(q, card) || q.chars(card).has_subtype(planet)
    })
}

const BASIC_LAND_OR_PLANET: select::Part = select::part(&(basic_land_or_planet as cantrip_api::select::SelectFn), 0, "");
const SEARCH: Selection = Selection::of(select::up_to(1), &[BASIC_LAND_OR_PLANET]).called("basic land or Planet");

/// "Search your library for a basic land card or Planet card, put it onto the battlefield tapped, then shuffle."
/// The found card enters tapped before the library is shuffled (CR 701.23, 701.24).
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    library::find_to_battlefield(g, item.controller, item.object, SEARCH, true)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[], effect),
    ways: &[beam_me_up::way(mana("{1}{G}"), "Beam me up {1}{G}")],
    ..oracle::FINAL_FRONTIER
};
