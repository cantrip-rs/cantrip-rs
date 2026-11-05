use cantrip_api::*;
use cantrip_mechanics::targets;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;

/// The slot for one opponent of "for each opponent, ... up to one target creature that player controls" (CR 601.2c, 115.1):
/// a creature that opponent controls. The core makes a slot of its own for each opponent (`TargetSpec::EACH_OPPONENT`) and
/// says which one this is.
pub(crate) fn creature_the_slots_opponent_controls(q: &dyn Query, _: &Asker, t: Target) -> bool {
    let Target::Object(id) = t else { return false };
    targets::controlled_by_slot_opponent(q, id) && q.chars(id).is(CardType::Creature)
}

const FOR_EACH_OPPONENT: TargetSpec = target(creature_the_slots_opponent_controls)
    .between(0, TargetSpec::EACH_OPPONENT)
    .labelled("Creature {player} controls to tap and put a stun counter on");

/// "... tap up to one target creature that player controls. Put a stun counter on each of those creatures" (CR 115.1d):
/// the targets still legal when it resolves are tapped together and each gets a stun counter (CR 122.1).
fn tap_and_stun(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let creatures = targets::legal_objects(g, item);
    g.tap_together(&creatures);
    let puts: Vec<(ObjectId, CounterKind, i64)> = creatures.into_iter().map(|id| (id, CounterKind::Stun, 1)).collect();
    g.add_counters_together(&puts);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            THIS_ENTERS,
            tap_and_stun,
            "When Hapatra enters, for each opponent, tap up to one target creature that player controls. Put a stun counter on each of those creatures.",
        )
        .targets(&[FOR_EACH_OPPONENT]),
        ability(cost("{2}{U}"), effects::untap_targets, "{2}{U}: Untap target creature.").targets(&[targets::CREATURE]),
    ],
    ..oracle::HAPATRA_THE_DESERT_FROST
};
