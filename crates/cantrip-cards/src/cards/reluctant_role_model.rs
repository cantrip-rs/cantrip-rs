use cantrip_api::*;
use cantrip_mechanics::{choose, effects, targets, triggers};
use cantrip_oracle as oracle;

/// "Put a flying, lifelink, or +1/+1 counter on it" (CR 122.1b, 608.2d): this creature's controller chooses the kind as the
/// ability resolves, so it is asked even when only one could matter; a creature that has left the battlefield gets nothing.
fn put_a_flying_lifelink_or_plus_one_counter_on_this(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let card = item.ability.card;
    let kind = match choose::one_of(g, item.controller, item.object, vec![Label::Card(card, 0), Label::Card(card, 1), Label::Card(card, 2)])? {
        0 => CounterKind::Flying,
        1 => CounterKind::Lifelink,
        _ => CounterKind::PlusOne,
    };
    g.add_counters(item.object, kind, 1);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            triggers::YOUR_SECOND_MAIN_PHASE,
            put_a_flying_lifelink_or_plus_one_counter_on_this,
            "Survival \u{2014} At the beginning of your second main phase, if this creature is tapped, put a flying, lifelink, or +1/+1 counter on it.",
        )
        .condition(triggers::this_is_tapped),
        triggered(
            triggers::CREATURE_YOU_CONTROL_WITH_COUNTERS_DIES,
            effects::put_the_subjects_counters_on_target,
            "Whenever this creature or another creature you control dies, if it had counters on it, put those counters on up to one target creature.",
        )
        .targets(&[up_to_one(targets::CREATURE.filter)]),
    ],
    labels: &["Flying counter", "Lifelink counter", "+1/+1 counter"],
    ..oracle::RELUCTANT_ROLE_MODEL
};
