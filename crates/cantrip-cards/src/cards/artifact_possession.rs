use cantrip_api::*;
use cantrip_mechanics::{effects, targets, triggers};
use cantrip_oracle as oracle;

/// The enchanted artifact becomes tapped (CR 603.2); keep the artifact as the trigger's subject.
fn enchanted_artifact_tapped(q: &dyn Query, asker: &Asker, event: &Event) -> Option<Fired> {
    let Event::Tapped { permanent } = *event else { return None };
    if q.state().obj(asker.source).attached_to == Some(permanent) {
        Fired::about(Target::Object(permanent))
    } else {
        None
    }
}

/// An ability of the enchanted artifact is activated without {T} in its cost (CR 603.2, 602.2).
fn enchanted_artifact_activated(q: &dyn Query, asker: &Asker, event: &Event) -> Option<Fired> {
    let (source, _, def) = triggers::activation(q, event)?;
    if def.from == ZoneKind::Battlefield && !def.cost.tap && q.state().obj(asker.source).attached_to == Some(source) {
        Fired::about(Target::Object(source))
    } else {
        None
    }
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            when_asked(EventMask::TAPPED, enchanted_artifact_tapped),
            effects::damage_subject_controller::<2>,
            "Whenever enchanted artifact becomes tapped or a player activates an ability of enchanted artifact without {T} in its activation cost, this Aura deals 2 damage to that artifact's controller.",
        ),
        triggered(
            when_asked(EventMask::ABILITY_ACTIVATED, enchanted_artifact_activated),
            effects::damage_subject_controller::<2>,
            "Whenever enchanted artifact becomes tapped or a player activates an ability of enchanted artifact without {T} in its activation cost, this Aura deals 2 damage to that artifact's controller.",
        ),
    ],
    enchant: Some(targets::ARTIFACT),
    ..oracle::ARTIFACT_POSSESSION
};
