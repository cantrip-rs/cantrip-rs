use cantrip_api::*;
use cantrip_mechanics::{greatest_power, statics};
use cantrip_oracle as oracle;

/// "This spell costs {X} less to cast, where X is the greatest power among creatures you control" (CR 601.2f, 118.7a): a
/// generic reduction, read as the cost is determined; only casting the spell is reduced, never a special action taken for
/// the card (CR 116.2).
fn reduction(q: &dyn Query, _: ObjectId, p: PlayerId, costed: Costed, _: &[Target]) -> CostDelta {
    let x = greatest_power::among_creatures_you_control(q, p);
    if matches!(costed, Costed::Spell(_)) { CostDelta::less(x.min(255) as u8) } else { CostDelta::NONE }
}

pub static CARD: CardDef = CardDef {
    own_cost_changes: &[reduction],
    statics: &[static_ability(
        Layer::Ability,
        statics::other_creatures_you_control,
        statics::have_trample,
        "Other creatures you control have trample.",
    )],
    ..oracle::GHALTA_THE_UNSTOPPABLE
};
