use cantrip_api::*;
use cantrip_mechanics::library::find_to_hand;
use cantrip_mechanics::select::{self, Selection, up_to};
use cantrip_mechanics::effects;
use cantrip_mechanics::lands::is_basic_land_card;
use cantrip_oracle as oracle;

/// The selection rule of "a basic land card and a Shrine card" (CR 701.23, 205.4a, 205.3i): some of one each, and a card may be
/// added while they still would be.
fn basic_land_and_shrine(q: &dyn Query, choice: &Choice, _: u32) -> Verdict {
    let shrine = q.words(choice.source).subtype(Subtype::Shrine);
    select::one_of_each(choice, |card| is_basic_land_card(q, card), |card| q.chars(card).has_subtype(shrine))
}

/// "A basic land card and a Shrine card" (CR 701.23, 205.4a): one search finding up to one of each; a card that is neither can't
/// be taken, and neither can a second of one kind.
const A_BASIC_LAND_CARD_AND_A_SHRINE_CARD: Selection = Selection::of(
    up_to(2),
    &[select::part(&(basic_land_and_shrine as SelectFn), 0, "a basic land card and a Shrine card")],
)
.called("basic land and Shrine");

/// "Search your library for a basic land card. If this spell was kicked, instead search your library for a basic land card and a
/// Shrine card. Reveal those cards, put them into your hand, then shuffle. You gain 2 life." (CR 701.23, 702.33, 119.3): the
/// kicked search is one search for both cards, found or not (each may be left in the library, CR 701.23b), revealed and put into
/// the hand together, and the library shuffled once.
fn search_then_gain_two_life(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let rule = if item.kicked > 0 { A_BASIC_LAND_CARD_AND_A_SHRINE_CARD } else { select::BASIC_LAND_CARD };
    find_to_hand(g, item.controller, item.object, rule, true)?;
    effects::gain_life(g, item, 2)
}

pub static CARD: CardDef = CardDef {
    kicker: kicker(mana("{2}")),
    spell: spell(&[], search_then_gain_two_life),
    ..oracle::AANGS_JOURNEY
};
