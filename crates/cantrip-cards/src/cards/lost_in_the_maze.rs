use cantrip_api::*;
use cantrip_mechanics::{statics, targets, triggers::THIS_ENTERS};
use cantrip_oracle as oracle;

/// "When this enchantment enters, tap X target creatures. Put a stun counter on each of those creatures you don't control."
/// X is the X the enchantment was cast with and counts the targets (CR 107.3m, 601.2b-c). The creatures still legal as it
/// resolves are tapped (CR 701.26a, 608.2b); a creature that was already tapped still gets its counter (CR 122.1d). Whom
/// "you don't control" is judged as the ability resolves.
fn tap_and_stun(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let creatures = targets::legal_objects(g, item);
    g.tap_together(&creatures);
    let puts: Vec<(ObjectId, CounterKind, i64)> = creatures
        .into_iter()
        .filter(|&creature| g.obj(creature).controller != item.controller)
        .map(|creature| (creature, CounterKind::Stun, 1))
        .collect();
    g.add_counters_together(&puts);
    Ok(())
}

/// "Tapped creatures you control": a creature, as the earlier layers made it, you control and that is tapped now (CR 613.1f).
fn your_tapped_creatures(q: &dyn LayerQuery, source: ObjectId, affected: ObjectId, so_far: &Chars) -> bool {
    statics::your_creatures(q, source, affected, so_far) && q.state().obj(affected).tapped
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        THIS_ENTERS,
        tap_and_stun,
        "When this enchantment enters, tap X target creatures. Put a stun counter on each of those creatures you don't control.",
    )
    .targets(&[targets::CREATURE.counted()])],
    statics: &[static_ability(Layer::Ability, your_tapped_creatures, statics::have_hexproof, "Tapped creatures you control have hexproof.")],
    ..oracle::LOST_IN_THE_MAZE
};
