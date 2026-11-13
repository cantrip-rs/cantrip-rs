use cantrip_api::*;
use cantrip_mechanics::crew::crew;
use cantrip_mechanics::targets::{self, filters};
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_mechanics::{affinity, effects};
use cantrip_oracle as oracle;

/// The slot for one opponent of "for each player, destroy up to one target creature that player controls" (CR 601.2c, 115.1): a
/// creature that opponent controls. The core makes a slot of its own for each opponent (`TargetSpec::EACH_OPPONENT`) and says
/// which one this is.
fn creature_the_slots_opponent_controls(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    let Target::Object(id) = t else { return false };
    targets::controlled_by_slot_opponent(q, id) && filters::creature(q, asker, id)
}

/// "For each player" is the controller (a slot of their own) and each opponent (a slot each, in seat order).
const YOUR_OWN: TargetSpec = targets::CREATURE_YOU_CONTROL.between(0, 1).labelled("Creature you control to destroy");
const FOR_EACH_OPPONENT: TargetSpec =
    target(creature_the_slots_opponent_controls).between(0, TargetSpec::EACH_OPPONENT).labelled("Creature {player} controls to destroy");

/// "... destroy up to one target creature that player controls. If a creature you controlled was destroyed this way, put two +1/+1
/// counters on this Vehicle." The targets still legal are destroyed together (CR 608.2b, 701.8); only a creature that was destroyed
/// counts (an indestructible one stays), and whose it was is read before it dies (CR 608.2h).
fn destroy_for_each_player(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let objects = targets::legal_objects(g, item);
    let yours: Vec<ObjectId> = objects.iter().copied().filter(|&o| g.controller(o) == item.controller).collect();
    let destroyed = g.destroy_all(&objects);
    g.end_sentence();
    if yours.iter().any(|o| destroyed.permanents().contains(o)) {
        effects::two_plus_one_counters_on_this(g, item)?;
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    own_cost_changes: &[affinity::for_artifacts],
    abilities: &[
        triggered(
            THIS_ENTERS,
            destroy_for_each_player,
            "When this Vehicle enters, for each player, destroy up to one target creature that player controls. If a creature you controlled was destroyed this way, put two +1/+1 counters on this Vehicle.",
        )
        .targets(&[YOUR_OWN, FOR_EACH_OPPONENT]),
        crew(2, "Crew 2 (Tap any number of creatures you control with total power 2 or more: This Vehicle becomes an artifact creature until end of turn.)"),
    ],
    ..oracle::DEMONIC_JUNKER
};
