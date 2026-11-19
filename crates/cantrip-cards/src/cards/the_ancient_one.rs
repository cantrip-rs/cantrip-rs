use cantrip_api::*;
use cantrip_mechanics::{effects, hand, targets};
use cantrip_oracle as oracle;

/// "Descend 8 \u{2014} ... can't attack ... unless there are eight or more permanent cards in your graveyard" (CR 508.1c, 207.2c,
/// 110.4a): asked of this creature as attackers are declared; once it is attacking it keeps attacking whatever leaves the graveyard.
fn attacks_only_with_descend_8(q: &dyn Query, asker: &Asker, attacker: ObjectId, _: Target) -> bool {
    asker.source != attacker || targets::you_have_permanent_cards_in_graveyard::<8>(q, asker)
}

/// "... can't ... block unless there are eight or more permanent cards in your graveyard" (CR 509.1b).
fn blocks_only_with_descend_8(q: &dyn Query, source: ObjectId, blocker: ObjectId, _: ObjectId) -> bool {
    source != blocker || targets::permanent_cards_in_graveyard(q, q.controller(source)) >= 8
}

/// "Draw a card, then discard a card. When you discard a card this way, target player mills cards equal to its mana value"
/// (CR 121.1, 701.9, 603.12): the reflexive ability triggers if a card was discarded, even one a replacement put into a hidden zone
/// unrevealed (CR 701.9c), and carries that card's mana value as it is where it went: 0 when that is undefined.
fn loot_then_mill_for_it(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let discarded = effects::loot_discarding(g, item.controller, item.object)?;
    if let Some(&card) = discarded.first() {
        let value = if hand::discarded_chars(&*g, card).is_some() { g.mana_value(card) } else { 0 };
        let this = g.card_id(&CARD);
        g.reflexive_trigger_counted(item.object, item.controller, AbilityRef::delayed(this, 0), Target::None, value);
    }
    Ok(())
}

/// "Target player mills cards equal to its mana value" (CR 701.17a, 202.3e): the count the reflexive ability carries, the discarded
/// card's mana value where it went ({X} counts as 0); the player is milled only if still a legal target as this resolves (CR 608.2b).
fn target_player_mills_its_mana_value(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Some(player) = targets::player(g, item, 0) {
        g.mill(player, item.x as u32);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        cost("{2}{U}{B}"),
        loot_then_mill_for_it,
        "{2}{U}{B}: Draw a card, then discard a card. When you discard a card this way, target player mills cards equal to its mana value.",
    )],
    delayed: &[delayed_trigger(target_player_mills_its_mana_value, "When you discard a card this way, target player mills cards equal to its mana value.")
        .targets(&[targets::PLAYER])],
    attack_rules: &[attacks_only_with_descend_8],
    block_rules: &[blocks_only_with_descend_8],
    ..oracle::THE_ANCIENT_ONE
};
