use cantrip_api::*;
use cantrip_mechanics::library::{ask_library_order, reveal_until};
use cantrip_mechanics::replacements::{instead_of_drawing, you_would_draw_a_card};
use cantrip_mechanics::choose;
use cantrip_oracle as oracle;

/// "You may instead ..." (CR 614.1a): asked of Abundance's controller as their draw would be replaced.
static INSTEAD: Question = Question::new(
    "RevealUntilKind",
    "choose land or nonland and reveal cards until you reveal one of that kind, instead of drawing a card?",
    "That card goes into your hand and the other cards revealed on the bottom of your library. If you don't, the draw goes on as it was.",
    "Reveal instead",
    "Don't",
);

/// "... choose land or nonland and reveal cards from the top of your library until you reveal a card of the chosen kind.
/// Put that card into your hand and put all other cards revealed this way on the bottom of your library in any order."
/// (CR 701.20, 401.4) The card is put into the hand, not drawn (CR 121.5). With no card of the kind in the library the whole
/// library is revealed and put back in the order the player chooses (its ruling); an empty one reveals nothing.
fn reveal_instead(g: &mut dyn Game, asker: &Asker, _: &Proposal, _: &Proposal) {
    let _ = reveal_until_kind(g, asker.controller, asker.source);
}

fn reveal_until_kind(g: &mut dyn Game, player: PlayerId, source: ObjectId) -> Result<(), Suspend> {
    // The labels are Abundance's own.
    let card = g.card_id(&CARD);
    let land = choose::one_of(g, player, source, vec![Label::Card(card, 0), Label::Card(card, 1)])? == 0;
    let (found, rest) = reveal_until(g, player, 1, &|q, c| q.chars(c).is(CardType::Land) == land);
    if !found.is_empty() {
        g.return_to_hand(&found);
    }
    let bottom = ask_library_order(g, player, source, &rest, DestinationKind::LibraryBottom)?;
    g.arrange_library(player, &[], &bottom);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    // "If you would draw a card, you may instead ..." (CR 121.6, 614.1a)
    replacements: &[replacement(
        ReplacementKind::Draw,
        you_would_draw_a_card,
        instead_of_drawing,
        "may reveal until a land or nonland card and put it into hand instead",
    )
    .then(reveal_instead)
    .optional(&INSTEAD)],
    labels: &["Land", "Nonland"],
    ..oracle::ABUNDANCE
};
