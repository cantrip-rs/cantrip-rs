use cantrip_api::*;
use cantrip_mechanics::library::search_hand_and_or_library;
use cantrip_mechanics::select::{self, Part, Selection};
use cantrip_mechanics::{lands, triggers, typecycling};
use cantrip_oracle as oracle;

/// "A Dragon card": a card with the creature type Dragon, read through the words of the text that asks for it (CR 612).
fn dragon(q: &dyn Query, choice: &Choice, _: u32) -> Verdict {
    let dragon = q.words(choice.source).subtype(Subtype::Dragon);
    Verdict::each(choice, |card| q.chars(card).has_subtype(dragon))
}

const DRAGON: Part = select::part(&(dragon as SelectFn), 0, "");

/// A search for "a Dragon card", which may be left unfound (CR 701.23b).
const DRAGON_CARD: Selection = Selection::of(select::up_to(1), &[DRAGON]).called("Dragon");

const SEARCH: Question = Question::new(
    "SearchLibrary",
    "search your hand and library for a Dragon card?",
    "A Dragon card you find, in either, goes onto the battlefield, and you shuffle. If you don't, you may still take one from your hand",
    "Search both",
    "Hand only",
);

/// "Whenever a Mountain you control enters, put a quest counter on this enchantment. If it has six or more quest counters on
/// it, sacrifice it. If you do, search your hand and/or library for a Dragon card and put it onto the battlefield. If you
/// search your library this way, shuffle." Three instructions in order (CR 608.2c): the counter, the sacrifice if it has
/// six or more (the counters it has now, and none if it has already left, which sacrifices nothing, CR 400.7), and, if it was
/// sacrificed, the search. A Dragon card found that is no permanent card stays where it is (CR 110.4); the library is shuffled
/// if it was searched, whether or not a card was found there (CR 701.23, 701.24).
fn quest(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.add_counters(item.object, CounterKind::Quest, 1);
    g.end_sentence();
    if g.obj(item.object).counters(CounterKind::Quest) < 6 || g.sacrifice(item.controller, &[item.object]).is_empty() {
        return Ok(());
    }
    g.end_sentence();
    let you = item.controller;
    let (found, searched_library) = search_hand_and_or_library(g, you, item.object, SearchDestination::Battlefield, Purpose::PutOntoBattlefield, &SEARCH, DRAGON_CARD)?;
    // "A Dragon card": at most one, from the hand or from the library.
    if let Some(&dragon) = found.first() { g.put_onto_battlefield(dragon, you, false)?; }
    g.end_sentence();
    if searched_library {
        g.shuffle(you);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            when_asked(EventMask::ZONE_CHANGE, triggers::of_subtype_you_control_enters::<{ lands::sub(Subtype::Mountain) }>),
            quest,
            "Whenever a Mountain you control enters, put a quest counter on this enchantment. If it has six or more quest counters on it, sacrifice it. If you do, search your hand and/or library for a Dragon card and put it onto the battlefield. If you search your library this way, shuffle.",
        ),
        typecycling::ability(Subtype::Mountain, mana("{2}"), "Mountaincycling {2} ({2}, Discard this card: Search your library for a Mountain card, reveal it, put it into your hand, then shuffle.)"),
    ],
    ..oracle::LAST_LIGHT_OF_DURINS_DAY
};
