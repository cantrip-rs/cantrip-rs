use cantrip_api::*;
use cantrip_mechanics::triggers::BEGINNING_OF_YOUR_COMBAT;
use cantrip_mechanics::{effects, increment, targets};
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
    effects::may_pay_x_then_when_you_do(g, item, &CARD, &PAY_X, Target::None)
}

/// "Move X +1/+1 counters from this creature onto another target creature" (CR 122.5): each counter is removed from this
/// creature and then put onto the target, so the abilities that care about either apply (the rulings). With fewer than X
/// on it, it moves as many as it has. Nothing moves if the target is illegal by now (CR 608.2b), or if it can't have
/// the counters put on it (Solemnity, CR 122.5): none come off this creature then.
fn move_counters(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    g.move_counters(item.object, creature, CounterKind::PlusOne, item.x as i32);
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
