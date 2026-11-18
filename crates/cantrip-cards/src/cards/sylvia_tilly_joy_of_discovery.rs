use cantrip_api::*;
use cantrip_mechanics::triggers::{BEGINNING_OF_YOUR_COMBAT, THIS_ENTERS};
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

const PAY: Question = Question::new(
    "Pay",
    "pay {cost} and exile a card from your graveyard to put a +1/+1 counter on target creature?",
    "",
    "Pay {cost}",
    "Don't pay",
);

/// "When Sylvia Tilly enters, surveil 2." (CR 701.25a).
fn surveil_two(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::surveil(g, item, 2)
}

/// "When you do, put a +1/+1 counter on target creature" (CR 122.1, 608.2b): the target as the reflexive ability resolves.
fn put_the_counter(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    g.add_counters(creature, CounterKind::PlusOne, 1);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(THIS_ENTERS, surveil_two, "When Sylvia Tilly enters, surveil 2."),
        triggered(
            BEGINNING_OF_YOUR_COMBAT,
            |g, item| effects::may_pay_then_when_you_do(g, item, &CARD, cost("{1}").and(exile_from_graveyard(1)), &PAY),
            "At the beginning of combat on your turn, you may pay {1} and exile a card from your graveyard. When you do, put a +1/+1 counter on target creature.",
        ),
    ],
    delayed: &[delayed_trigger(put_the_counter, "When you do, put a +1/+1 counter on target creature.").targets(&[targets::CREATURE])],
    ..oracle::SYLVIA_TILLY_JOY_OF_DISCOVERY
};
