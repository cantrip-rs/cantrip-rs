use cantrip_api::*;
use cantrip_mechanics::{choose, library, select, targets, triggers};
use cantrip_oracle as oracle;

/// "Exile target creature and all Auras attached to it. Note the number and kind of counters that were on that creature.
/// When this artifact leaves the battlefield or becomes untapped, return that exiled card ..." (CR 406.2, 101.4, 603.7):
/// the creature and every Aura attached to it, whoever controls it, are exiled together; nothing if the target is no
/// longer legal (CR 608.2b). The delayed triggered ability is created whatever was exiled (CR 603.7a) - a token creature
/// has ceased to exist by the time it resolves and leaves its Auras in exile forever (the rulings) - and watches this
/// artifact as the object the ability came from: one that left the battlefield before this resolved never triggers it
/// (CR 603.7a, the ruling). It carries the creature as it was, whose counters are the noted ones as it last existed (CR
/// 113.7a) and whose cards in exile it finds then (CR 400.7, 730.3c), then the Aura cards that reached exile.
fn exile_and_note(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    let aura = g.words(item.object).subtype(Subtype::Aura);
    let auras: Vec<ObjectId> = g
        .state()
        .battlefield()
        .iter()
        .copied()
        .filter(|&id| g.attached_to(id) == Some(creature) && g.chars(id).has_subtype(aura))
        .collect();
    let mut exiling = vec![creature];
    exiling.extend(&auras);
    g.exile_all(&exiling);
    let mut carried = vec![creature];
    carried.extend(auras.iter().filter_map(|&a| g.found_after_move(a)).filter(|&card| library::is_still_exiled(g, card)));
    // With no subject of its own, a "when [event]" ability watches its source (`Delayed::watched`): this artifact.
    g.delayed_trigger_about(DelayedWhen::WhenEvent, item.object, item.controller, AbilityRef::delayed(item.ability.card, 0), &carried);
    Ok(())
}

/// "When this artifact leaves the battlefield or becomes untapped" (CR 603.7b, 603.6c, 701.26b): the next time the
/// artifact it watches leaves the battlefield or untaps, whichever comes first, and only once.
fn leaves_or_untaps(q: &dyn Query, coffin: ObjectId, event: &Event) -> Option<Fired> {
    match event {
        Event::Untapped { permanent } if *permanent == coffin => Fired::PLAIN,
        _ => triggers::this_leaves(q, coffin, event),
    }
}

/// "Return that exiled card to the battlefield under its owner's control tapped with the noted number and kind of counters
/// on it. If you do, return the other exiled cards to the battlefield under their owner's control attached to that
/// permanent." The creature's card, if it is still in exile (CR 603.7c, 400.7) - every card of a merged creature, each
/// the same way (CR 730.3c) - enters tapped (the ruling: it is not tapped afterwards) with the counters it had as it was
/// exiled (CR 122.6), whatever it is now (the ruling). Only if one did, the other exiled cards still in exile follow under
/// their owners' control, all at once, each attached to it. With several returned permanents an Aura could enchant, its
/// controller chooses which (CR 303.4d), each owner as theirs enters, the active player first (CR 101.4, 101.4b). One no
/// returned permanent can hold is named one anyway and the move decides: an Aura stays exiled (CR 303.4i, the ruling),
/// and anything else - a bestowed creature, which is no Aura in exile (CR 400.7, 702.103b) - enters unattached (CR
/// 303.4h).
fn return_them(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let carried = g.event_objects(item).to_vec();
    let creature = carried[0];
    let noted: Vec<(CounterKind, i32)> =
        CounterKind::ALL.iter().map(|&kind| (kind, g.obj(creature).counters(kind))).filter(|&(_, n)| n > 0).collect();
    let cards: Vec<(ObjectId, PlayerId, bool)> = g
        .found_all_after_move(creature)
        .into_iter()
        .filter(|&card| library::is_still_exiled(g, card))
        .map(|card| (card, g.obj(card).owner, true))
        .collect();
    let returned = g.put_all_onto_battlefield_with_counters(&cards, &noted)?;
    let hosts: Vec<ObjectId> = returned.into_iter().filter(|&p| g.is_on_battlefield(p)).collect();
    if hosts.is_empty() {
        return Ok(());
    }
    let apnap: Vec<PlayerId> = g.state().apnap().collect();
    let mut others = library::still_exiled(g, &carried[1..]);
    others.sort_by_key(|&card| apnap.iter().position(|&p| p == g.obj(card).owner));
    let mut attaching = Vec::new();
    for card in others {
        let owner = g.obj(card).owner;
        let able: Vec<ObjectId> = hosts.iter().copied().filter(|&h| g.could_enter_attached(card, owner, h)).collect();
        // The one it may be attached to, its owner's pick of several, or - none being possible - any one for the move to judge.
        let chosen =
            if able.len() > 1 { choose::select(g, owner, card, Purpose::Attach, able, select::cards(select::exactly(1)))? } else { able };
        attaching.push((card, owner, chosen.first().copied().unwrap_or(hosts[0])));
    }
    g.put_all_onto_battlefield_attached(&attaching)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    // "You may choose not to untap this artifact during your untap step." (CR 502.3)
    may_not_untap: true,
    abilities: &[ability(
        cost("{3}").and(TAP),
        exile_and_note,
        "{3}, {T}: Exile target creature and all Auras attached to it. Note the number and kind of counters that were on that creature. When this artifact leaves the battlefield or becomes untapped, return that exiled card to the battlefield under its owner's control tapped with the noted number and kind of counters on it. If you do, return the other exiled cards to the battlefield under their owner's control attached to that permanent.",
    )
    .targets(&[targets::CREATURE])],
    delayed: &[triggered(
        when(EventMask(EventMask::ZONE_CHANGE.0 | EventMask::UNTAPPED.0), leaves_or_untaps),
        return_them,
        "When this artifact leaves the battlefield or becomes untapped, return that exiled card to the battlefield under its owner's control tapped with the noted number and kind of counters on it. If you do, return the other exiled cards to the battlefield under their owner's control attached to that permanent.",
    )],
    ..oracle::TAWNOSS_COFFIN
};
