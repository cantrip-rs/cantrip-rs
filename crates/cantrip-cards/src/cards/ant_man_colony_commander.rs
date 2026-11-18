use cantrip_api::*;
use cantrip_mechanics::triggers::THIS_ATTACKS;
use cantrip_mechanics::{effects, targets, tokens};
use cantrip_oracle as oracle;

const PAY: Question = Question::new("Pay", "pay {cost} to put a +1/+1 counter on target creature?", "", "Pay {cost}", "Don't pay");

/// The reflexive ability's counter, put by Ant-Man's controller (CR 122.6) on the target if it is still a legal target
/// (CR 608.2b).
fn put_the_counter(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    g.add_counters(creature, CounterKind::PlusOne, 1);
    Ok(())
}

/// "Whenever you put a +1/+1 counter on a creature" (CR 122.6, 122.6a, 603.2): +1/+1 counters put on a creature, on the battlefield or
/// as it enters, by this creature's controller - the player who put them is the event's, so counters an opponent puts do not trigger it.
fn you_put_a_counter_on_a_creature(q: &dyn Query, source: ObjectId, event: &Event) -> Option<Fired> {
    match event {
        Event::CountersPut { object, kind: CounterKind::PlusOne, amount, by, .. }
            if *amount > 0 && *by == q.controller(source) && q.is_on_battlefield(*object) && q.chars(*object).is(CardType::Creature) =>
        {
            Fired::PLAIN
        }
        _ => None,
    }
}

fn create_an_insect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create(g, item.controller, &tokens::INSECT, 1)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(THIS_ATTACKS, |g, item| effects::may_pay_then_when_you_do(g, item, &CARD, cost("{1}"), &PAY), "Whenever Ant-Man attacks, you may pay {1}. When you do, put a +1/+1 counter on target creature."),
        triggered(
            when(EventMask::COUNTERS, you_put_a_counter_on_a_creature),
            create_an_insect,
            "Whenever you put a +1/+1 counter on a creature, create a 1/1 green Insect creature token. This ability triggers only once each turn.",
        )
        .triggers_each_turn(1),
    ],
    delayed: &[delayed_trigger(put_the_counter, "When you do, put a +1/+1 counter on target creature.").targets(&[targets::CREATURE])],
    ..oracle::ANT_MAN_COLONY_COMMANDER
};

pub static EXTRA: &[&CardDef] = &[&tokens::INSECT];
