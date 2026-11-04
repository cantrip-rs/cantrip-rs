use cantrip_api::*;
use cantrip_mechanics::effects::{self, plus_one_counter_on_this};
use cantrip_mechanics::triggers::{THIS_DIES, YOU_GAIN_LIFE};
use cantrip_oracle as oracle;

/// "Put X +1/+1 counters on each legendary creature you control, where X is the number of +1/+1 counters on this creature"
/// (CR 122.1, 603.10a, 608.2h): X is the count it had as it last was on the battlefield, which stays on its object after it
/// died; the counters go on every legendary creature the controller has as the ability resolves, all at once.
fn counters_on_each_legendary_creature(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let x = g.obj(item.object).counters(CounterKind::PlusOne);
    let legends: Vec<ObjectId> = g
        .permanents_of(item.controller)
        .into_iter()
        .filter(|&id| {
            let c = g.chars(id);
            c.is(CardType::Creature) && c.supertypes.legendary
        })
        .collect();
    effects::counters_on_each(g, &legends, CounterKind::PlusOne, x);
    Ok(())
}

/// Lifelink is in the characteristics. "Whenever you gain life" triggers once for each life-gain event, not for each point.
pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(YOU_GAIN_LIFE, plus_one_counter_on_this, "Whenever you gain life, put a +1/+1 counter on Aerith Gainsborough."),
        triggered(
            THIS_DIES,
            counters_on_each_legendary_creature,
            "When Aerith Gainsborough dies, put X +1/+1 counters on each legendary creature you control, where X is the number of +1/+1 counters on Aerith Gainsborough.",
        ),
    ],
    ..oracle::AERITH_GAINSBOROUGH
};
