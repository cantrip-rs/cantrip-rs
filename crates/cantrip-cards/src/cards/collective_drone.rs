use cantrip_api::*;
use cantrip_mechanics::lands;
use cantrip_oracle as oracle;

/// "Collective Drone's power is equal to the number of artifact creatures you control" (CR 604.3, 208.2a, 613.4a):
/// a characteristic-defining ability, so it applies in every zone and includes the Drone itself while it is an artifact
/// creature.
fn power_is_artifact_creatures_you_control(q: &dyn LayerQuery, source: ObjectId, chars: &mut Chars) {
    chars.power = Some(lands::count_you_control(q, source, &|c| {
        c.is(CardType::Artifact) && c.is(CardType::Creature)
    }));
}

pub static CARD: CardDef = CardDef {
    statics: &[characteristic_defining(
        Layer::PtDefine,
        power_is_artifact_creatures_you_control,
        "Collective Drone's power is equal to the number of artifact creatures you control.",
    )
    .lost_pt(0, 2)],
    ..oracle::COLLECTIVE_DRONE
};
