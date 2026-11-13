use cantrip_api::*;
use cantrip_mechanics::{effects, lands, targets};
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_oracle as oracle;

/// "Target Mount you control becomes saddled until end of turn" (CR 702.171b, 608.2b): if the Mount is still a legal target.
fn target_mount_becomes_saddled(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(mount) = targets::object(g, item, 0) else { return Ok(()) };
    g.become_saddled(mount);
    Ok(())
}

/// "Target Vehicle you control becomes an artifact creature until end of turn" (CR 301.7b, 611.2a, 608.2b): it stays a Vehicle
/// and an artifact, and is a creature as well.
fn target_vehicle_becomes_creature(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(vehicle) = targets::object(g, item, 0) else { return Ok(()) };
    g.add_effect(item.object, vehicle, EffectKind::AddTypes(TypeSet::of(&[CardType::Creature])), Duration::EndOfTurn);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(THIS_ENTERS, |g, item| effects::draw(g, item, 1), "When this artifact enters, draw a card."),
        ability(
            cost("{2}, {T}"),
            target_mount_becomes_saddled,
            "{2}, {T}: Target Mount you control becomes saddled until end of turn. Activate only as a sorcery.",
        )
        .targets(&[target(targets::permanent_of_subtype_you_control::<{ lands::sub(Subtype::Mount) }>)])
        .sorcery_speed(),
        ability(
            cost("{2}, {T}"),
            target_vehicle_becomes_creature,
            "{2}, {T}: Target Vehicle you control becomes an artifact creature until end of turn.",
        )
        .targets(&[targets::VEHICLE_YOU_CONTROL]),
    ],
    ..oracle::GUIDELIGHT_MATRIX
};
