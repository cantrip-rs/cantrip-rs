use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "Its power is 6 or greater" (CR 208.1): the power the layers give it now, a negative one counting as it is.
fn power_is_six_or_greater(q: &dyn Query, source: ObjectId) -> bool {
    q.chars(source).power.is_some_and(|power| power >= 6)
}

/// "This creature can't attack ... unless its power is 6 or greater" (CR 508.1c): asked of this creature as attackers are declared.
/// Once declared it keeps attacking whatever happens to its power.
fn attacks_only_with_power_six(q: &dyn Query, source: ObjectId, attacker: ObjectId) -> bool {
    source != attacker || power_is_six_or_greater(q, source)
}

/// "This creature can't ... block unless its power is 6 or greater" (CR 509.1b).
fn blocks_only_with_power_six(q: &dyn Query, source: ObjectId, blocker: ObjectId, _: ObjectId) -> bool {
    source != blocker || power_is_six_or_greater(q, source)
}

/// "Draw a card. Put a +1/+1 counter on this creature" (CR 121.1, 122.1): the artifact was sacrificed as a cost.
fn draw_and_grow(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::draw(g, item, 1)?;
    g.end_sentence();
    g.add_counters(item.object, CounterKind::PlusOne, 1);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        cost("{T}").and(sacrifice(1, targets::filters::another_artifact)),
        draw_and_grow,
        "{T}, Sacrifice another artifact: Draw a card. Put a +1/+1 counter on this creature.",
    )],
    attack_rules: &[attacks_only_with_power_six],
    block_rules: &[blocks_only_with_power_six],
    ..oracle::TECHNODROME
};
