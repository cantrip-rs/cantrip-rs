use cantrip_api::*;
use cantrip_mechanics::{choose, creature_cards_in_graveyard, select, targets, triggers};
use cantrip_oracle as oracle;

/// "Target player reveals a number of cards from their hand equal to one plus the number of creature cards in your graveyard. You
/// choose one of them. That player discards that card." (CR 701.20a, 701.9, 608.2d): the number is counted once, as the ability
/// resolves, in Klaw's controller's graveyard (tokens are not cards); the target player chooses which cards of their hand to
/// reveal, all of them when the hand holds no more than that; then Klaw's controller chooses one of the revealed cards, and the
/// target player discards it. An empty hand reveals nothing and nothing is discarded; nothing happens if the player is no
/// longer a legal target (CR 608.2b).
fn sonic_attack(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(player) = targets::player(g, item, 0) else { return Ok(()) };
    let hand = g.state().zone(Zone::hand(player)).to_vec();
    if hand.is_empty() {
        return Ok(());
    }
    let count = 1 + cantrip_api::count_u32(creature_cards_in_graveyard(g, item.controller));
    let rule = Selection::new(select::as_many_as_can(count));
    let revealed = choose::select(g, player, item.object, Purpose::Reveal, hand, rule)?;
    g.reveal(player, &revealed);
    let chosen = choose::select(g, item.controller, item.object, Purpose::Discard, revealed, Selection::new(select::exactly(1)))?;
    g.discard_revealed(&chosen);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::THIS_ENTERS,
        sonic_attack,
        "Sonic Attack \u{2014} When Klaw enters, target player reveals a number of cards from their hand equal to one plus the number of creature cards in your graveyard. You choose one of them. That player discards that card.",
    )
    .targets(&[targets::PLAYER])],
    ..oracle::KLAW_SONIC_SUBJUGATOR
};
