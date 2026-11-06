use cantrip_api::*;
use cantrip_mechanics::{effects, select, targets, ninjutsu, triggers::DEALS_COMBAT_DAMAGE_TO_PLAYER_EVENT};
use cantrip_oracle as oracle;

/// "Whenever this creature deals combat damage to a player, you may discard a creature card. When you do, destroy target
/// creature or planeswalker that player controls." (CR 701.9, 603.12): the card is chosen as the ability resolves, the player
/// chooses one creature card of their hand or none, and the reflexive trigger is queued once a card is discarded.
fn discard_a_creature_card(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if effects::may_discard(g, item.controller, item.object, select::CREATURE_CARD)? {
        let this = g.card_id(&CARD);
        g.reflexive_trigger(item.object, item.controller, AbilityRef::delayed(this, 0), item.subject);
    }
    Ok(())
}

/// "Target creature or planeswalker that player controls": the player this creature dealt combat damage to, the trigger's
/// subject, whether or not the creature is still in combat.
fn creature_or_planeswalker_that_player_controls(q: &dyn Query, _: &Asker, t: Target) -> bool {
    let Target::Object(id) = t else { return false };
    let c = q.chars(id);
    targets::controlled_by_that_player(q, id) && (c.is(CardType::Creature) || c.is(CardType::Planeswalker))
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ninjutsu::ninjutsu(mana("{1}{B}"), "Ninjutsu {1}{B} ({1}{B}, Return an unblocked attacker you control to hand: Put this card onto the battlefield from your hand tapped and attacking.)"),
        triggered(
            DEALS_COMBAT_DAMAGE_TO_PLAYER_EVENT,
            discard_a_creature_card,
            "Whenever this creature deals combat damage to a player, you may discard a creature card. When you do, destroy target creature or planeswalker that player controls.",
        ),
    ],
    delayed: &[delayed_trigger(cantrip_mechanics::effects::destroy_first_target, "When you do, destroy target creature or planeswalker that player controls.")
        .targets(&[target(creature_or_planeswalker_that_player_controls)])],
    ..oracle::DOKUCHI_SILENCER
};
