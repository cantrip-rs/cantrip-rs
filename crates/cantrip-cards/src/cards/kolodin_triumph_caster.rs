use cantrip_api::*;
use cantrip_mechanics::statics::{have_haste, mounts_and_vehicles_you_control};
use cantrip_mechanics::{lands, triggers};
use cantrip_oracle as oracle;

/// "Whenever a Mount you control enters" (CR 603.6a): about the Mount that entered.
const MOUNT_YOU_CONTROL_ENTERS: Trigger = when_asked(EventMask::ZONE_CHANGE, triggers::of_subtype_you_control_enters::<{ lands::sub(Subtype::Mount) }>);

/// "It becomes saddled until end of turn" (CR 702.171b), "it" the Mount that entered, the trigger's subject. A Mount that has left
/// the battlefield is a new object (CR 400.7) and becomes nothing.
fn it_becomes_saddled(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Some(mount) = item.subject.object() {
        g.become_saddled(mount);
    }
    Ok(())
}

/// "It becomes an artifact creature until end of turn" (CR 205.1b, 301.7), "it" the Vehicle that entered, the trigger's subject;
/// one that has left the battlefield becomes nothing.
fn it_becomes_an_artifact_creature(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Some(vehicle) = item.subject.object() {
        instruction(g, |g| {
            g.add_effect(item.object, vehicle, EffectKind::AddTypes(TypeSet::of(&[CardType::Artifact, CardType::Creature])), Duration::EndOfTurn);
        });
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    statics: &[static_ability(Layer::Ability, mounts_and_vehicles_you_control, have_haste, "Mounts and Vehicles you control have haste.")],
    abilities: &[
        triggered(MOUNT_YOU_CONTROL_ENTERS, it_becomes_saddled, "Whenever a Mount you control enters, it becomes saddled until end of turn."),
        triggered(
            triggers::VEHICLE_YOU_CONTROL_ENTERS,
            it_becomes_an_artifact_creature,
            "Whenever a Vehicle you control enters, it becomes an artifact creature until end of turn.",
        ),
    ],
    ..oracle::KOLODIN_TRIUMPH_CASTER
};
