use cantrip_api::*;
use cantrip_mechanics::an_opponent_controls_more_lands;
use cantrip_mechanics::library::{ask_may_search, search};
use cantrip_mechanics::select::{self, Selection};
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_oracle as oracle;

const ASK: Question = Question::new("SearchLibrary", "search your library for a Plains card?", "It enters tapped; if you search at all, you shuffle afterwards", "Search", "Don't search");

/// "A Plains card" (CR 205.3i, 305.6): a land card with the subtype Plains, basic or not, read through this card's own words.
const PLAINS_CARD: Selection = Selection::of(select::up_to(1), &[select::subtype_card(Subtype::Plains)]).called("Plains");

/// One pass of the process: "you may search your library for a Plains card and put it onto the battlefield tapped"
/// (CR 701.23). Whether a search was made, so the caller can shuffle: a declined or forbidden search is no search.
fn may_search_for_a_plains(g: &mut dyn Game, item: &StackItem) -> Result<bool, Suspend> {
    let player = item.controller;
    if !ask_may_search(g, player, item.object, &ASK)? {
        return Ok(false);
    }
    let found = search(g, player, player, item.object, SearchDestination::BattlefieldTapped, PLAINS_CARD)?;
    if let Some(&card) = found.first() {
        g.put_onto_battlefield(card, player, true)?;
    }
    Ok(true)
}

/// "When this creature enters, if an opponent controls more lands than you, you may search your library for a Plains card
/// and put it onto the battlefield tapped. Then if an opponent controls more lands than you, repeat this process once. If you
/// search your library this way, shuffle." (CR 603.4, 701.23, 701.24): the intervening "if" is the ability's condition, the
/// second check is made again after the first pass (a declined first pass leaves the lands as they were), and the library is
/// shuffled once at the end, and only if a search was made.
fn search_for_plains_and_maybe_repeat(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let asker = Asker::new(item.object, item.controller);
    let mut searched = may_search_for_a_plains(g, item)?;
    let repeat = {
        let q: &dyn Query = g;
        an_opponent_controls_more_lands(q, &asker)
    };
    if repeat {
        searched |= may_search_for_a_plains(g, item)?;
    }
    if searched {
        g.shuffle(item.controller);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        THIS_ENTERS,
        search_for_plains_and_maybe_repeat,
        "When this creature enters, if an opponent controls more lands than you, you may search your library for a Plains card and put it onto the battlefield tapped. Then if an opponent controls more lands than you, repeat this process once. If you search your library this way, shuffle.",
    )
    .condition(an_opponent_controls_more_lands)],
    ..oracle::CLAIM_JUMPER
};

pub static EXTRA: &[&CardDef] = &[];
