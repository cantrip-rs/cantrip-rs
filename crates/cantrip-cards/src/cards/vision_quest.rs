use cantrip_api::*;
use cantrip_mechanics::library::search_zone_and_or_library;
use cantrip_mechanics::{effects, select};
use cantrip_oracle as oracle;

const ASK: Question = Question::new(
    "SearchLibrary",
    "search your library and graveyard for an artifact creature card with mana value X or less?",
    "Then shuffle. If you don't, you search only your graveyard and must take a card that fits",
    "Search both",
    "Graveyard only",
);

/// "Search your library and/or graveyard for an artifact creature card with mana value X or less and put it onto the
/// battlefield with X additional +1/+1 counters on it. If X is 4 or greater, it gains haste until end of turn. If you search
/// your library this way, shuffle." (CR 701.23, 122.6, 702.10)
///
/// `library::search_zone_and_or_library` with the graveyard: the library is searched only if its owner chooses to, and then
/// the one card is chosen among both zones, or left unfound (CR 701.23b); a graveyard-only search is no library search (no
/// shuffle, CR 701.23a) and, the graveyard being public, finds a card that fits. An {X} in a library or graveyard is 0
/// (CR 202.3e), as `mana_value` reads it. The counters are part of how it enters (CR 122.6) and the haste is for a card that
/// did enter.
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let you = item.controller;
    let x = item.x as u32;
    let rule = Selection::new(select::up_to(1)).with(select::ARTIFACT).with(select::CREATURE).with(select::mana_value_at_most(x)).called("artifact creature");
    let (found, searched_library) =
        search_zone_and_or_library(g, you, item.object, &[Zone::graveyard(you)], SearchDestination::Battlefield, Purpose::PutOntoBattlefield, &ASK, rule)?;
    if let Some(&card) = found.first() {
        let counters = [(CounterKind::PlusOne, i64::from(i16::try_from(x).unwrap_or(i16::MAX)))];
        let entered = g.put_onto_battlefield_with_counters(card, you, false, &counters)?;
        if x >= 4 && g.is_on_battlefield(entered) {
            effects::gain_until_end_of_turn(g, item.object, entered, Keyword::Haste);
        }
    }
    if searched_library {
        g.shuffle(you);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef { spell: spell(&[], effect), ..oracle::VISION_QUEST };
