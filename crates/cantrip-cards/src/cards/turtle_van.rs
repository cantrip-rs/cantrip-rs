use cantrip_api::*;
use cantrip_mechanics::{crew::crew, double, targets, triggers::THIS_ATTACKS};
use cantrip_oracle as oracle;

/// "Put a +1/+1 counter on target creature that crewed it this turn. Then if that creature is a Mutant, Ninja, or Turtle,
/// double the number of +1/+1 counters on it." (CR 122.1, 701.10e.) The subtypes are read through the Vehicle's words
/// (CR 612).
fn counter_then_double(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    g.add_counters(creature, CounterKind::PlusOne, 1);
    g.end_sentence();
    let kinds = [Subtype::Mutant, Subtype::Ninja, Subtype::Turtle].map(|s| g.words(item.object).subtype(s));
    if kinds.iter().any(|&s| g.chars(creature).has_subtype(s)) {
        double::counters(g, &[creature], CounterKind::PlusOne);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            THIS_ATTACKS,
            counter_then_double,
            "Whenever this Vehicle attacks, put a +1/+1 counter on target creature that crewed it this turn. Then if that creature is a Mutant, Ninja, or Turtle, double the number of +1/+1 counters on it.",
        )
        .targets(&[targets::CREATURE_THAT_CREWED_IT]),
        crew(1, "Crew 1 (Tap any number of creatures you control with total power 1 or more: This Vehicle becomes an artifact creature until end of turn.)"),
    ],
    ..oracle::TURTLE_VAN
};
