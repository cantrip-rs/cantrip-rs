use cantrip_api::*;
use cantrip_mechanics::library::{ask_may_search, search};
use cantrip_mechanics::{choose, effects, select};
use cantrip_oracle as oracle;

const ASK: Question = Question::new("SearchLibrary", "search your library for an artifact creature card with mana value X or less?", "Then shuffle", "Search", "Don't search");

fn eligible(q: &dyn Query, card: ObjectId, x: u32) -> bool {
    let chars = q.chars(card);
    chars.is(CardType::Artifact) && chars.is(CardType::Creature) && q.mana_value(card) <= x
}

/// "Search your library and/or graveyard for an artifact creature card with mana value X or less and put it onto the
/// battlefield with X additional +1/+1 counters on it. If X is 4 or greater, it gains haste until end of turn. If you search
/// your library this way, shuffle." (CR 701.23, 122.6, 702.10)
///
/// The library is searched only if its owner chooses to ("and/or": a graveyard-only search is no library search, so there is no
/// shuffle then, CR 701.23a), and the card in it may be left unfound (CR 701.23b); the graveyard is a public zone, so every card
/// of it that fits is found. The one card put onto the battlefield is chosen among what each zone had. An {X} in a library or graveyard is 0 (CR 202.3e), as `mana_value` reads
/// it. The counters are part of how it enters (CR 122.6) and the haste is for a card that did enter.
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let you = item.controller;
    let x = item.x as u32;
    let mut options: Vec<ObjectId> = Vec::new();
    let searched_library = ask_may_search(g, you, item.object, &ASK)?;
    if searched_library {
        let library = cantrip_mechanics::library::searched_cards(g, you);
        let can: Vec<ObjectId> = library.iter().copied().filter(|&c| eligible(g, c, x)).collect();
        options.extend(search(g, you, you, item.object, SearchDestination::Battlefield, Selection::listed(select::up_to(1), &library, can))?);
    }
    options.extend(g.state().zone(Zone::graveyard(you)).iter().copied().filter(|&c| eligible(g, c, x)));
    let picked = match options.as_slice() {
        [] => None,
        &[only] => Some(only),
        _ => choose::select(g, you, item.object, Purpose::PutOntoBattlefield, options, select::cards(select::exactly(1)))?.first().copied(),
    };
    if let Some(card) = picked {
        let counters = [(CounterKind::PlusOne, i16::try_from(x).unwrap_or(i16::MAX))];
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
