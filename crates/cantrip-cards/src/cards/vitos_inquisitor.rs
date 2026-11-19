use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "Put a +1/+1 counter on this creature. It gains menace until end of turn" (CR 122.1, 611.2a, 702.111): two instructions. Once the
/// Inquisitor has left the battlefield (CR 400.7) it is a new object and neither applies.
fn plus_one_counter_on_this_and_it_gains_menace(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::plus_one_counter_on_this(g, item)?;
    effects::this_gains_until_end_of_turn(g, item, Keyword::Menace)
}

pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        cost("{B}").and(sacrifice(1, targets::filters::another_creature_or_artifact)),
        plus_one_counter_on_this_and_it_gains_menace,
        "{B}, Sacrifice another creature or artifact: Put a +1/+1 counter on this creature. It gains menace until end of turn.",
    )],
    ..oracle::VITOS_INQUISITOR
};
