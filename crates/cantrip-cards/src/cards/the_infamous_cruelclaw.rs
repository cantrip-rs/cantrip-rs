use cantrip_api::*;
use cantrip_mechanics::{effects, library, triggers};
use cantrip_oracle as oracle;

const CAST_BY_DISCARDING: Question = Question::new("CastByDiscarding", "cast it by discarding a card?", "Discard a card rather than pay its mana cost", "Cast it", "Don't cast it");

/// "You may cast that card by discarding a card rather than paying its mana cost" as the ability resolves (CR 608.2g, 118.9, 701.9):
/// the discard is the one alternative cost of the cast (CR 118.9a), its X 0 (CR 107.3b) and its additional costs paid as usual
/// (CR 118.8), its timing waived. Nothing is asked when `player` has no card in hand to discard or could not cast it at all (no
/// legal target, split second on the stack); asked, they may decline, and a cast they give up (CR 733.1) leaves it where it was and
/// discards nothing.
fn may_cast_by_discarding_a_card(g: &mut dyn Game, player: PlayerId, card: ObjectId) -> Result<(), Suspend> {
    if g.state().zone(Zone::hand(player)).is_empty()
        || !g.could_cast_during_resolution_by(card, player)
        || !effects::ask_may(g, player, card, Purpose::Ask(&CAST_BY_DISCARDING))?
    {
        return Ok(());
    }
    g.cast_during_resolution_as(card, CastTerms { discard: 1, caster: Some(player), ..CastTerms::default() })?;
    Ok(())
}

/// "Exile cards from the top of your library until you exile a nonland card. You may cast that card by discarding a card rather
/// than paying its mana cost." (CR 701.13a, 608.2g, 118.9): the exiled lands and the card stay exiled when it is not cast.
fn exile_until_nonland_then_may_cast_by_discarding(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Some(card) = library::exile_until_any_nonland(g, item.controller, None) {
        may_cast_by_discarding_a_card(g, item.controller, card)?;
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::DEALS_COMBAT_DAMAGE_TO_PLAYER_EVENT,
        exile_until_nonland_then_may_cast_by_discarding,
        "Whenever The Infamous Cruelclaw deals combat damage to a player, exile cards from the top of your library until you exile a nonland card. You may cast that card by discarding a card rather than paying its mana cost.",
    )],
    ..oracle::THE_INFAMOUS_CRUELCLAW
};
