use cantrip_api::*;
use cantrip_mechanics::statics::plus_pt;
use cantrip_oracle as oracle;

/// "Mounts and Vehicles you control" (CR 613.4c, 205.3m, 301.7): every permanent under the source's controller that the earlier
/// layers made a Mount or a Vehicle, creature or not, the types read through the words of the source's text (CR 612.2).
fn mounts_and_vehicles_you_control(q: &dyn LayerQuery, source: ObjectId, affected: ObjectId, so_far: &Chars) -> bool {
    let words = q.words(source);
    (so_far.has_subtype(words.subtype(Subtype::Mount)) || so_far.has_subtype(words.subtype(Subtype::Vehicle))) && q.controller(affected) == q.controller(source)
}

/// "This creature saddles Mounts and crews Vehicles as though its power were 2 greater" (CR 702.171c, 702.122b) are the generated
/// keywords `SaddlesAsThoughPowerTwoGreater` and `CrewsAsThoughPowerTwoGreater`, which the crew and saddle costs read.
pub static CARD: CardDef = CardDef {
    statics: &[static_ability(Layer::PtModify, mounts_and_vehicles_you_control, plus_pt::<1, 1>, "Mounts and Vehicles you control get +1/+1.")],
    ..oracle::CLOUDSPIRE_CAPTAIN
};
