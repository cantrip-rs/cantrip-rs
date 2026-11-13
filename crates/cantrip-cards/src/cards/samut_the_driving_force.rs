use cantrip_api::*;
use cantrip_mechanics::cost_reduction::noncreature_spells_you_cast;
use cantrip_mechanics::start_your_engines::speed;
use cantrip_mechanics::statics::other_creatures_you_control;
use cantrip_oracle as oracle;

/// "Other creatures you control get +X/+0, where X is your speed" (CR 613.4c, 702.179f): X is its controller's speed as the
/// layers are applied, 0 for a player with no speed.
fn plus_speed_power(q: &dyn LayerQuery, source: ObjectId, chars: &mut Chars) {
    let x = i32::from(speed(q.state(), q.state().obj(source).controller));
    chars.power = chars.power.map(|v| v + x);
}

/// "Noncreature spells you cast cost {X} less to cast, where X is your speed" (CR 601.2f, 118.7a): the speed of the source's
/// controller as the cost is determined.
fn noncreature_spells_cost_speed_less(q: &dyn Query, source: ObjectId, player: PlayerId, costed: Costed, _: &[Target]) -> CostDelta {
    noncreature_spells_you_cast(q, source, player, costed, u32::from(speed(q.state(), q.controller(source))))
}

pub static CARD: CardDef = CardDef {
    start_your_engines: true,
    statics: &[static_ability(Layer::PtModify, other_creatures_you_control, plus_speed_power, "Other creatures you control get +X/+0, where X is your speed.")],
    cost_changes: &[noncreature_spells_cost_speed_less],
    ..oracle::SAMUT_THE_DRIVING_FORCE
};
