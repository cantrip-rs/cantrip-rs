use cantrip_api::*;
use cantrip_mechanics::combat::choose_attack_target;
use cantrip_mechanics::{targets, triggers};
use cantrip_oracle as oracle;

const PAY: Question = Question::new(
    "Pay",
    "pay {cost} to return a creature card from your graveyard to the battlefield?",
    "It returns tapped and attacking with a finality counter on it",
    "Pay {cost}",
    "Don't pay",
);

/// "Whenever Grim Reaper attacks, you may pay {3}{B}. When you do, ..." (CR 118.12, 603.12): the payment is made as the ability
/// resolves, and only paying triggers the reflexive ability, which chooses its target as it goes on the stack. Mana for spells
/// can't pay it (CR 106.6).
fn may_pay_to_return(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if g.may_pay(item.controller, item.object, item.object, of_mana(mana("{3}{B}")), &PAY)? {
        let this = g.card_id(&CARD);
        g.reflexive_trigger(item.object, item.controller, AbilityRef::delayed(this, 0), Target::None);
    }
    Ok(())
}

/// "Return target creature card from your graveyard to the battlefield tapped and attacking with a finality counter on it."
/// (CR 508.4, 122.6, 614.1c): the card enters as a new object under its owner's control, which is its controller's, tapped, with
/// the counter, attacking a player or planeswalker its controller chooses, which need not be what Grim Reaper attacks; it was never
/// declared as an attacker (CR 508.4), so nothing that watches attackers being declared sees it. With nothing it can attack,
/// it just enters tapped with the counter. Nothing happens if the card has left the graveyard (CR 608.2b).
fn return_tapped_and_attacking(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(card) = targets::object(g, item, 0) else { return Ok(()) };
    let you = item.controller;
    let finality = [(CounterKind::Finality, 1)];
    // With nothing to attack (no defender), `put_onto_battlefield_attacking` only taps it.
    let defender = choose_attack_target(g, you, item.object)?.unwrap_or(Target::None);
    g.put_onto_battlefield_attacking(card, you, defender, &finality)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::THIS_ATTACKS,
        may_pay_to_return,
        "Whenever Grim Reaper attacks, you may pay {3}{B}. When you do, return target creature card from your graveyard to the battlefield tapped and attacking with a finality counter on it.",
    )],
    delayed: &[delayed_trigger(
        return_tapped_and_attacking,
        "When you do, return target creature card from your graveyard to the battlefield tapped and attacking with a finality counter on it.",
    )
    .targets(&[targets::CREATURE_CARD_IN_YOUR_GRAVEYARD])],
    ..oracle::GRIM_REAPER_LETHAL_LEGIONNAIRE
};
