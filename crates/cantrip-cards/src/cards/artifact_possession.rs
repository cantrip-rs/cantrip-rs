use cantrip_api::*;
use cantrip_mechanics::{effects, targets, triggers};
use cantrip_oracle as oracle;

/// "Whenever enchanted artifact becomes tapped or a player activates an ability of enchanted artifact without {T} in its
/// activation cost" (CR 603.2, 602.2): one triggered ability with two conditions, the enchanted artifact becoming tapped, or an
/// ability of it activated without {T} in its cost. The artifact is the trigger's subject. The `listens` mask is `TAPPED` and
/// `ABILITY_ACTIVATED`.
fn enchanted_artifact_tapped_or_activated(q: &dyn Query, asker: &Asker, event: &Event) -> Option<Fired> {
    let enchanted = q.state().obj(asker.source).attached_to;
    if let Event::Tapped { permanent, .. } = *event {
        return if enchanted == Some(permanent) { Fired::about(Target::Object(permanent)) } else { None };
    }
    let (source, _, def) = triggers::activation(q, event)?;
    if def.from == ZoneKind::Battlefield && !def.cost.tap && enchanted == Some(source) {
        Fired::about(Target::Object(source))
    } else {
        None
    }
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        when_asked(EventMask(EventMask::TAPPED.0 | EventMask::ABILITY_ACTIVATED.0), enchanted_artifact_tapped_or_activated),
        effects::damage_subject_controller::<2>,
        "Whenever enchanted artifact becomes tapped or a player activates an ability of enchanted artifact without {T} in its activation cost, this Aura deals 2 damage to that artifact's controller.",
    )],
    enchant: Some(targets::ARTIFACT),
    ..oracle::ARTIFACT_POSSESSION
};
