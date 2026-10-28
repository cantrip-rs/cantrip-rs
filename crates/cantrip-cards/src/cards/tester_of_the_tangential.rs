use cantrip_api::*;
use cantrip_mechanics::triggers::BEGINNING_OF_YOUR_COMBAT;
use cantrip_mechanics::{increment, targets};
use cantrip_oracle as oracle;

const PAY_X: Question = Question::new(
    "MoveCounters",
    "pay {X}?",
    "When you do, move X +1/+1 counters from this creature onto another target creature",
    "Pay {cost}",
    "Don't pay",
)
.no_default();

/// "At the beginning of combat on your turn, you may pay {X}. When you do, ..." X is the payer's to choose as they pay it
/// (CR 107.3f), 0 included; paying is doing it, whatever X, and the reflexive ability (CR 603.12) triggers carrying X.
/// Declining is not doing it: nothing triggers. The target is chosen when the reflexive ability goes on the stack.
fn pay_x(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let you = item.controller;
    if let Some(x) = g.may_pay_x(you, item.object, item.object, cost("{X}"), &PAY_X)? {
        let this = g.card_id(&CARD);
        g.reflexive_trigger_counted(item.object, you, AbilityRef::delayed(this, 0), Target::None, x);
    }
    Ok(())
}

/// "Move X +1/+1 counters from this creature onto another target creature" (CR 122.1): each counter is removed from this
/// creature and then put onto the target, so the abilities that care about either apply (the rulings). With fewer than X
/// on it, it moves as many as it has. Nothing moves if the target is illegal by now (CR 608.2b), and none come off this
/// creature then.
fn move_counters(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    // A creature holds at most i16::MAX counters, so an X past that moves them all just the same.
    let moved = g.remove_counters(item.object, CounterKind::PlusOne, i16::try_from(item.x).unwrap_or(i16::MAX));
    g.add_counters(creature, CounterKind::PlusOne, moved);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        increment::TRIGGER,
        triggered(BEGINNING_OF_YOUR_COMBAT, pay_x, "At the beginning of combat on your turn, you may pay {X}. When you do, move X +1/+1 counters from this creature onto another target creature."),
    ],
    delayed: &[delayed_trigger(move_counters, "When you do, move X +1/+1 counters from this creature onto another target creature.")
        .targets(&[targets::OTHER_CREATURE])],
    ..oracle::TESTER_OF_THE_TANGENTIAL
};
