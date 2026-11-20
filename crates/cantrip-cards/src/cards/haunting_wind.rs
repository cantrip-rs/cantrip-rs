use cantrip_api::*;
use cantrip_mechanics::{effects::ping_subject_controller, triggers};
use cantrip_oracle as oracle;

/// "Whenever an artifact becomes tapped (CR 603.2e) or a player activates an artifact's ability without {T} in its activation
/// cost (CR 602.2, 603.2)": the artifact is the subject.
fn artifact_tapped_or_ability_activated_without_tap(q: &dyn Query, _: &Asker, event: &Event) -> Option<Fired> {
    if let Event::Tapped { permanent, .. } = event {
        return if q.chars(*permanent).is(CardType::Artifact) { Fired::about(Target::Object(*permanent)) } else { None };
    }
    let (source, _, def) = triggers::activation(q, event)?;
    if def.from == ZoneKind::Battlefield && q.chars(source).is(CardType::Artifact) && !def.cost.tap {
        Fired::about(Target::Object(source))
    } else {
        None
    }
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            when_asked(
                EventMask(EventMask::TAPPED.0 | EventMask::ABILITY_ACTIVATED.0),
                artifact_tapped_or_ability_activated_without_tap,
            ),
            ping_subject_controller::<1>,
            "Whenever an artifact becomes tapped or a player activates an artifact's ability without {T} in its activation cost, this enchantment deals 1 damage to that artifact's controller.",
        ),
    ],
    ..oracle::HAUNTING_WIND
};
