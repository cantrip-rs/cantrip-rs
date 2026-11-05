use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_oracle as oracle;

/// "Whenever one or more creatures deal combat damage to you while you're the monarch" (CR 603.2c, 725.1): one event for each
/// creature that dealt some of it to the controller, its subject that creature, so the ability - being `once_per_batch` -
/// triggers once for the batch, its members those creatures. Only creatures deal combat damage (CR 510.1, 506.4), so the source is not type-checked.
fn creature_deals_combat_damage_to_you(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    match event {
        Event::Damage { source, target: Target::Player(p), kind, .. }
            if kind.is_combat() && *p == me.controller && q.state().player(me.controller).monarch =>
        {
            Fired::about(Target::Object(*source))
        }
        _ => None,
    }
}

/// "Tap those creatures and put a stun counter on each of them" (CR 701.26a, 122.1d, 608.2c): all of them are tapped, then
/// each is given its counter.
fn tap_them_and_stun(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let them = g.event_objects(item).to_vec();
    g.tap_together(&them);
    let puts: Vec<(ObjectId, CounterKind, i64)> = them.into_iter().map(|id| (id, CounterKind::Stun, 1)).collect();
    g.add_counters_together(&puts);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(THIS_ENTERS, effects::become_monarch, "When Tamiyo enters, you become the monarch."),
        triggered(
            when_asked(EventMask::DAMAGE, creature_deals_combat_damage_to_you),
            tap_them_and_stun,
            "Whenever one or more creatures deal combat damage to you while you're the monarch, tap those creatures and put a stun counter on each of them.",
        )
        .once_per_batch(),
    ],
    ..oracle::TAMIYO_UPRISER_CROWNED
};
