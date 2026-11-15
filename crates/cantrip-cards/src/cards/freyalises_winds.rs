use cantrip_api::*;
use cantrip_oracle as oracle;

/// "Whenever a permanent becomes tapped" (CR 701.26a, 603.2): any player's permanent, by any means - a cost, a tap for
/// mana, an attack, an effect. The permanent is the ability's subject, "it".
fn a_permanent_becomes_tapped(_: &dyn Query, _: ObjectId, event: &Event) -> Option<Fired> {
    let Event::Tapped { permanent } = event else { return None };
    Fired::about(Target::Object(*permanent))
}

/// "... put a wind counter on it." (CR 122.1, 122.6): on the permanent that became tapped, if it is still that object on the
/// battlefield (CR 400.7).
fn put_a_wind_counter_on_it(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(permanent) = item.subject.object() else { return Ok(()) };
    g.add_counters(permanent, CounterKind::Wind, 1);
    Ok(())
}

/// "If a permanent with a wind counter on it would untap during its controller's untap step" (CR 614.1a, 502.3): any
/// player's permanent, in the untap step of the player who controls it - not an untap by an effect or a cost, nor one
/// "during each other player's untap step" (Seedborn Muse). Only a permanent that would untap: one that doesn't untap
/// (Barl's Cage, "you may choose not to untap") never meets the effect and keeps its counters (the 2009-10-01 ruling).
fn would_untap_during_its_controllers_untap_step(q: &dyn Query, _: &Asker, proposal: &Proposal) -> bool {
    matches!(proposal, Proposal::Untap(u) if u.untap_step == Some(u.controller) && q.obj(u.permanent).counters(CounterKind::Wind) > 0)
}

/// "... remove all wind counters from it instead." (CR 614.1a, 122.1): it stays tapped, and every wind counter it has comes
/// off.
fn remove_all_wind_counters_instead(q: &dyn Query, _: &Asker, proposal: &mut Proposal) {
    if let Proposal::Untap(u) = proposal {
        u.untaps = false;
        u.remove[CounterKind::Wind as usize] = q.obj(u.permanent).counters(CounterKind::Wind);
    }
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        when(EventMask::TAPPED, a_permanent_becomes_tapped),
        put_a_wind_counter_on_it,
        "Whenever a permanent becomes tapped, put a wind counter on it.",
    )],
    replacements: &[replacement(
        ReplacementKind::Untap,
        would_untap_during_its_controllers_untap_step,
        remove_all_wind_counters_instead,
        "remove all wind counters from it instead of untapping it",
    )],
    ..oracle::FREYALISES_WINDS
};
