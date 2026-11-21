use cantrip_api::*;
use cantrip_mechanics::{earthbend, triggers};
use cantrip_oracle as oracle;

/// "When this creature enters, earthbend 1, then earthbend 1." (CR 701.66a, 608.2c, 115.3): two instructions, each with its own
/// "target land you control" (slots 0 and 1, which may be the same land), carried out in order.
fn earthbend_twice(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    earthbend::earthbend_target(g, item, 0, 1)?;
    earthbend::earthbend_target(g, item, 1, 1)
}

/// "Each opponent loses X life and you gain X life, where X is the number of creatures you control with +1/+1 counters on them"
/// (CR 119.3, 608.2h): counted as the ability resolves, in the layers; the opponents lose it together.
fn drain_for_creatures_with_counters(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let x = g
        .permanents_of(item.controller)
        .into_iter()
        .filter(|&id| g.chars(id).is(CardType::Creature) && g.obj(id).counters(CounterKind::PlusOne) > 0)
        .count() as i32;
    g.each_opponent_loses_life(item.controller, x);
    g.gain_life(item.controller, x);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(triggers::THIS_ENTERS, earthbend_twice, "When this creature enters, earthbend 1, then earthbend 1.").targets(&[earthbend::TARGET, earthbend::TARGET]),
        triggered(
            triggers::THIS_ATTACKS,
            drain_for_creatures_with_counters,
            "Whenever this creature attacks, each opponent loses X life and you gain X life, where X is the number of creatures you control with +1/+1 counters on them.",
        ),
    ],
    delayed: &[earthbend::RETURN],
    ..oracle::DAI_LI_AGENTS
};
