use cantrip_api::*;
use cantrip_mechanics::statics::plus_pt;
use cantrip_oracle as oracle;

/// "Mounts and Vehicles you control" (CR 613.1f, 205.3m, 301.7): every permanent under the source's controller that the earlier
/// layers made a Mount or a Vehicle, creature or not, the types read through the words of the source's text (CR 612.2).
fn mounts_and_vehicles_you_control(q: &dyn LayerQuery, source: ObjectId, affected: ObjectId, so_far: &Chars) -> bool {
    let words = q.words(source);
    (so_far.has_subtype(words.subtype(Subtype::Mount)) || so_far.has_subtype(words.subtype(Subtype::Vehicle))) && q.controller(affected) == q.controller(source)
}

/// "This creature saddles Mounts and crews Vehicles as though its power were 2 greater" (CR 702.171c, 702.122b): asked as the
/// creature is tapped for a crew, saddle or station cost, of the permanent being crewed, saddled or stationed; a Spacecraft
/// it stations counts its own power.
fn two_greater_for_mounts_and_vehicles(q: &dyn Query, asker: &Asker, power: i32, permanent: ObjectId) -> i32 {
    let words = q.words(asker.source);
    let subtypes = q.chars(permanent).subtypes;
    if subtypes.has(words.subtype(Subtype::Vehicle)) || subtypes.has(words.subtype(Subtype::Mount)) {
        power + 2
    } else {
        power
    }
}

pub static CARD: CardDef = CardDef {
    statics: &[static_ability(Layer::PtModify, mounts_and_vehicles_you_control, plus_pt::<1, 1>, "Mounts and Vehicles you control get +1/+1.")],
    tap_power: &[two_greater_for_mounts_and_vehicles],
    ..oracle::CLOUDSPIRE_CAPTAIN
};
