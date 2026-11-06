use cantrip_api::*;
use cantrip_mechanics::{library, targets};
use cantrip_oracle as oracle;

/// Reveal the opponent's hand, then discard a randomly selected creature card (CR 701.20a, 701.9a).
fn reveal_and_discard(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(opponent) = targets::player(g, item, 0) else { return Ok(()) };
    let hand = g.state().zone(Zone::hand(opponent)).to_vec();
    g.reveal(opponent, &hand);
    let creatures: Vec<ObjectId> = hand.into_iter().filter(|&id| g.chars(id).is(CardType::Creature)).collect();
    if let Some(card) = library::random_of(g, &creatures) {
        g.discard_revealed(&[card]);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        cost("{B}{B}{B}").and(TAP),
        reveal_and_discard,
        "{B}{B}{B}, {T}: Target opponent reveals their hand and discards a creature card at random. Activate only during your turn.",
    ).targets(&[targets::OPPONENT]).timing(Timing::OwnTurn)],
    ..oracle::RAG_MAN
};
