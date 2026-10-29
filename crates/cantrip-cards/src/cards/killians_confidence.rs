use cantrip_api::*;
use cantrip_mechanics::triggers::creature_you_control_deals_combat_damage_to_player;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "Target creature gets +1/+1 until end of turn. Draw a card."
fn pump_and_draw(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::target_creature_gets::<1, 1>(g, item)?;
    g.end_sentence();
    effects::draw(g, item, 1)
}

const ASK: Question = Question::new("ReturnToHand", "pay {cost} to return {subject} to your hand?", "It is in your graveyard", "Pay {cost}", "Leave it");

/// "You may pay {W/B}. If you do, return this card from your graveyard to your hand." Paying is optional (CR 118.12); a card that
/// has left the graveyard by the time the ability resolves is a new object and is not returned (CR 400.7).
fn pay_to_return(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let card = item.object;
    if g.may_pay(item.controller, card, card, cost("{W/B}"), &ASK)? {
        effects::return_this_to_hand(g, item)?;
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE], pump_and_draw),
    abilities: &[
        // "One or more creatures ... deal combat damage to a player" (CR 603.2c): one trigger for each player the batch damaged.
        // It works only from the graveyard (CR 113.6b, 113.6m).
        triggered(
            when_asked(EventMask::DAMAGE, creature_you_control_deals_combat_damage_to_player),
            pay_to_return,
            "Whenever one or more creatures you control deal combat damage to a player, you may pay {W/B}. If you do, return this card from your graveyard to your hand.",
        )
        .sums_per_subject()
        .functions_in(IN_A_GRAVEYARD),
    ],
    ..oracle::KILLIANS_CONFIDENCE
};
