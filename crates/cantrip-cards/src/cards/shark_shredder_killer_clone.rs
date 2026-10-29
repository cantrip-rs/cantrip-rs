use cantrip_api::*;
use cantrip_mechanics::triggers::DEALS_COMBAT_DAMAGE_TO_PLAYER_SUBJECT;
use cantrip_mechanics::{sneak, targets};
use cantrip_oracle as oracle;

/// "Put up to one target creature card from that player's graveyard onto the battlefield under your control. It enters tapped
/// and attacking that player." (CR 508.4, 110.2a): attacking, but never declared as an attacker, so no attack trigger sees it.
/// The player is the trigger's subject, the one the combat damage was dealt to.
fn reanimate_attacking(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(card) = targets::object(g, item, 0) else { return Ok(()) };
    g.put_onto_battlefield_attacking(card, item.controller, item.subject, &[])?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    ways: &[sneak::way(mana("{3}{B}{B}"), "Sneak {3}{B}{B}")],
    abilities: &[triggered(
        DEALS_COMBAT_DAMAGE_TO_PLAYER_SUBJECT,
        reanimate_attacking,
        "Whenever Shark Shredder deals combat damage to a player, put up to one target creature card from that player's graveyard onto the battlefield under your control. It enters tapped and attacking that player.",
    )
    .targets(&[targets::UP_TO_ONE_CREATURE_CARD_IN_THAT_PLAYERS_GRAVEYARD])],
    ..oracle::SHARK_SHREDDER_KILLER_CLONE
};
