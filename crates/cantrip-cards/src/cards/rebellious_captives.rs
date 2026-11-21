use cantrip_api::*;
use cantrip_mechanics::{earthbend, effects};
use cantrip_oracle as oracle;

/// "Put two +1/+1 counters on this creature, then earthbend 2" (CR 701.66a): the counters go on the creature first; the land is
/// the ability's target, chosen as it was activated and checked again as it resolves, and earthbends even when the creature
/// has left the battlefield (the counters then go nowhere).
fn grow_then_earthbend_two(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::two_plus_one_counters_on_this(g, item)?;
    earthbend::effect::<2>(g, item)
}

pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        cost("{6}"),
        grow_then_earthbend_two,
        "Exhaust \u{2014} {6}: Put two +1/+1 counters on this creature, then earthbend 2. (Target land you control becomes a 0/0 creature with haste that's still a land. Put two +1/+1 counters on it. When it dies or is exiled, return it to the battlefield tapped. Activate each exhaust ability only once.)",
    )
    .targets(&[earthbend::TARGET])
    .exhaust()],
    delayed: &[earthbend::RETURN],
    ..oracle::REBELLIOUS_CAPTIVES
};
