use cantrip_api::*;
use cantrip_mechanics::{statics, triggers};
use cantrip_oracle as oracle;

/// The upkeep ability puts a counter on the source before determining X (CR 608.2c, 122.1).
fn upkeep(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    cantrip_mechanics::effects::plus_one_counter_on_this(g, item)?;
    g.end_sentence();
    // If the source has left the battlefield, the object's last known counters are used.
    let x = g.state().obj(item.object).counters(CounterKind::PlusOne) as u32;
    let paid = g.may_pay(
        item.controller, item.object, item.object, Cost::mana(generic(x)), &PAY_QUESTION,
    )?;
    g.end_sentence();
    if !paid {
        g.tap_together(&[item.object]);
        g.end_sentence();
        g.damage_together(&[(item.object, Target::Player(item.controller), x)]);
        g.end_sentence();
    }
    Ok(())
}

static PAY_QUESTION: Question = Question::new(
    "Pay",
    "pay {cost}?",
    "",
    "Pay {cost}",
    "Decline",
);

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            triggers::YOUR_UPKEEP,
            upkeep,
            "At the beginning of your upkeep, put a +1/+1 counter on this creature. Then you may pay {X}, where X is the number of +1/+1 counters on it. If you don't, tap this creature and it deals X damage to you.",
        ),
    ],
    statics: &[
        static_ability(
            Layer::Ability,
            statics::itself,
            statics::attacks_each_combat_if_able,
            "This creature attacks each combat if able.",
        ),
    ],
    ..oracle::PRIMORDIAL_OOZE
};
