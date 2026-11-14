use cantrip_api::*;
use cantrip_mechanics::statics;
use cantrip_oracle as oracle;

/// "Other tapped creatures you control" (CR 613.1f, layer 6): every other creature its controller controls, as the earlier layers
/// made it, that is tapped as the layers are worked out (re-asked on every pass, so untapping loses the hexproof at once).
fn other_tapped_creatures_you_control(q: &dyn LayerQuery, source: ObjectId, affected: ObjectId, so_far: &Chars) -> bool {
    q.state().obj(affected).tapped && statics::other_creatures_you_control(q, source, affected, so_far)
}

/// Flash, convoke and double strike come from the printed characteristics (CR 702.8, 702.51, 702.4); "other tapped creatures
/// you control have hexproof" (CR 702.11b) is a layer-6 static ability.
pub static CARD: CardDef = CardDef {
    statics: &[static_ability(
        Layer::Ability,
        other_tapped_creatures_you_control,
        statics::have_hexproof,
        "Other tapped creatures you control have hexproof.",
    )],
    ..oracle::THE_WANDERING_RESCUER
};
