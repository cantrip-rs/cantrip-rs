//! "This spell costs {1} less to cast for each ..." where the card is not affinity (CR 702.41a is the keyword, and only the cards
//! that print the word use `affinity`): a cost reduction (CR 601.2f, 118.7) that counts a pool of the caster's.

use cantrip_api::*;

/// "This spell costs {1} less to cast for each instant and sorcery card in your graveyard" (CR 601.2f, 118.7a): counted as
/// the cost is determined, the cards as they are in the graveyard. The count is clamped to a byte, which loses nothing: it
/// reduces generic mana, and a cost's generic part is itself a byte (docs/rules-exceptions.md RX-5). Eddymurk Crab and The
/// Dawning Archaic print it.
pub fn for_each_instant_and_sorcery_in_your_graveyard(q: &dyn Query, _: ObjectId, player: PlayerId, costed: Costed, _: &[Target]) -> CostDelta {
    // Only the cost of casting it: it has no abilities to activate, and "this spell" is a spell.
    let spell = matches!(costed, Costed::Spell(_));
    let cards = q.state().zone(Zone::graveyard(player)).iter().filter(|&&id| spell && (q.chars(id).is(CardType::Instant) || q.chars(id).is(CardType::Sorcery))).count();
    CostDelta::less(cards.min(255) as u8)
}
