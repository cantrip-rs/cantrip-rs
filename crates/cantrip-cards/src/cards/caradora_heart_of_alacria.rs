use cantrip_api::*;
use cantrip_mechanics::library::{ask_may_search, find_to_hand};
use cantrip_mechanics::replacements::{plus_one_counters_applies, plus_one_counters_apply_to_proposal};
use cantrip_mechanics::{lands, select, triggers};
use cantrip_oracle as oracle;

/// "A Mount or Vehicle card", which may be left unfound (CR 701.23b).
const MOUNT_OR_VEHICLE_CARD: Selection = Selection::of(
    select::up_to(1),
    &[select::subtype_card_of_either::<{ lands::sub(Subtype::Mount) }, { lands::sub(Subtype::Vehicle) }>()],
)
.called("Mount or Vehicle");

const ASK: Question = Question::new(
    "SearchLibrary",
    "search your library for a Mount or Vehicle card?",
    "Reveal it and put it into your hand, then shuffle",
    "Search",
    "Don't search",
);

/// "When Caradora enters, you may search your library for a Mount or Vehicle card, reveal it, put it into your hand, then
/// shuffle." (CR 701.23): asked first, and a declined search shuffles nothing.
fn search_library_mount_or_vehicle_card(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if ask_may_search(g, item.controller, item.object, &ASK)? {
        find_to_hand(g, item.controller, item.object, MOUNT_OR_VEHICLE_CARD, true)?;
    }
    Ok(())
}

/// "If one or more +1/+1 counters would be put on a creature or Vehicle you control, that many plus one +1/+1 counters are put
/// on it instead." (CR 122.6, 614.1a.) A Vehicle is an artifact whether or not it is a creature, read through the words of this
/// card's text (CR 612.2).
fn applies(q: &dyn Query, asker: &Asker, proposal: &Proposal) -> bool {
    let vehicle = q.words(asker.source).subtype(Subtype::Vehicle);
    plus_one_counters_applies(q, asker, proposal, CounterKind::PlusOne, &[CardType::Creature])
        || (plus_one_counters_applies(q, asker, proposal, CounterKind::PlusOne, &[CardType::Artifact])
            && matches!(proposal, Proposal::Counters(c) if q.chars(c.object).has_subtype(vehicle)))
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::THIS_ENTERS,
        search_library_mount_or_vehicle_card,
        "When Caradora enters, you may search your library for a Mount or Vehicle card, reveal it, put it into your hand, then shuffle.",
    )],
    replacements: &[replacement(
        ReplacementKind::Counters,
        applies,
        plus_one_counters_apply_to_proposal,
        "If one or more +1/+1 counters would be put on a creature or Vehicle you control, that many plus one +1/+1 counters are put on it instead.",
    )],
    ..oracle::CARADORA_HEART_OF_ALACRIA
};
