use cantrip_api::*;
use cantrip_mechanics::effects::gain_until_end_of_turn;
use cantrip_mechanics::targets::{self, other_creature_you_control_filter};
use cantrip_oracle as oracle;

/// "Another target attacking creature you control" (CR 115.1, 506.4, 109.5): a creature under your control, other than this one,
/// that is attacking when it is chosen.
fn another_attacking_creature_you_control(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    other_creature_you_control_filter(q, asker, t) && matches!(t, Target::Object(id) if q.is_attacking(id))
}

/// "Put a +1/+1 counter on another target attacking creature you control. If that creature is a Cat, it gains first strike until end
/// of turn." (CR 122.1a, 205.3m, 611.2a, 702.7): both lost if the target is no longer legal as it resolves (CR 608.2b).
fn counter_and_first_strike_if_a_cat(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    g.add_counters(creature, CounterKind::PlusOne, 1);
    g.end_sentence();
    let cat = g.words(item.object).subtype(Subtype::Cat);
    if g.chars(creature).has_subtype(cat) {
        gain_until_end_of_turn(g, item.object, creature, Keyword::FirstStrike);
        g.end_sentence();
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        cost("{3}{W}"),
        counter_and_first_strike_if_a_cat,
        "{3}{W}: Put a +1/+1 counter on another target attacking creature you control. If that creature is a Cat, it gains first strike until end of turn. (It deals combat damage before creatures without first strike.)",
    )
    .targets(&[target(another_attacking_creature_you_control)])],
    ..oracle::INGENIOUS_LEONIN
};
