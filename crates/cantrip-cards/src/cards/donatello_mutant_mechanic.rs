use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "Whenever an artifact you control is put into a graveyard from the battlefield, if it had counters on it" (CR 603.6c, 603.10a,
/// 603.4): looked back on, so the artifact is judged as it last was on the battlefield - an artifact then, under the controller
/// it last had, with counters of any kind. Each artifact is its own event, so artifacts dying together trigger once each, and
/// this permanent leaving in the same event sees them. The trigger is about that artifact, whose counters are put on the target.
fn artifact_with_counters_put_into_a_graveyard(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    match event {
        Event::ZoneChange { old, from, to, .. }
            if from.is(ZoneKind::Battlefield)
                && to.is(ZoneKind::Graveyard)
                && q.chars(*old).is(CardType::Artifact)
                && q.obj(*old).controller == me.controller
                && q.obj(*old).counter_array().iter().any(|&n| n != 0) =>
        {
            Fired::about(Target::Object(*old))
        }
        _ => None,
    }
}

/// "Put those counters on up to one target artifact or creature you control": every kind the artifact had, as it last was.
fn put_those_counters(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Target::Object(artifact) = item.subject else { return Ok(()) };
    effects::put_counters_of_on_target(g, item, artifact)
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(
            TAP,
            effects::three_counters_and_becomes_robot,
            "{T}: Put three +1/+1 counters on target artifact you control. If it isn't a creature, it becomes a 0/0 Robot creature in addition to its other types. Activate only as a sorcery.",
        )
        .targets(&[targets::ARTIFACT_YOU_CONTROL])
        .sorcery_speed(),
        triggered(
            when_asked(EventMask::ZONE_CHANGE, artifact_with_counters_put_into_a_graveyard),
            put_those_counters,
            "Whenever an artifact you control is put into a graveyard from the battlefield, if it had counters on it, put those counters on up to one target artifact or creature you control.",
        )
        .targets(&[up_to_one(targets::ARTIFACT_OR_CREATURE_YOU_CONTROL.filter)]),
    ],
    ..oracle::DONATELLO_MUTANT_MECHANIC
};
