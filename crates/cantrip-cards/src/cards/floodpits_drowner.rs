use cantrip_api::*;
use cantrip_mechanics::{effects, targets, triggers::THIS_ENTERS};
use cantrip_oracle as oracle;

/// "Target creature with a stun counter on it" (CR 115.1, 122.1d).
fn creature_with_a_stun_counter(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    (targets::CREATURE.filter)(q, asker, t) && matches!(t, Target::Object(id) if q.obj(id).counters(CounterKind::Stun) > 0)
}

/// "Shuffle this creature and target creature with a stun counter on it into their owners' libraries" (CR 701.24a): both
/// go in at once and each owner's library is shuffled. The Drowner stays out of it if it has left the battlefield by now
/// (CR 400.7); the ability does nothing at all when its target is illegal (CR 608.2b).
fn shuffle_away(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Some(creature) = targets::object(g, item, 0) {
        // Nothing in the Oracle text keeps the Drowner from being its own target: it would then be one object, shuffled once.
        let mut objects = vec![item.object, creature];
        objects.dedup();
        g.put_into_library(&objects, LibraryPlace::ShuffledIn);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(THIS_ENTERS, effects::tap_and_stun::<1>, "When Floodpits Drowner enters, tap target creature an opponent controls and put a stun counter on it.")
            .targets(&[targets::CREATURE_AN_OPPONENT_CONTROLS]),
        ability(
            cost("{1}{U}").and(TAP),
            shuffle_away,
            "{1}{U}, {T}: Shuffle Floodpits Drowner and target creature with a stun counter on it into their owners' libraries.",
        )
        .targets(&[target(creature_with_a_stun_counter)]),
    ],
    ..oracle::FLOODPITS_DROWNER
};
