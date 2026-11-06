use cantrip_api::*;
use cantrip_mechanics::lands;
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;

fn cost_reduction(q: &dyn Query, asker: &Asker, _: Costed, _: &[Target]) -> CostDelta {
    let lands = lands::differently_named_lands(q, asker.controller) as i32;
    CostDelta::less(lands as u32)
}

pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        cost("{6}"),
        effects::pump_self::<2, 2>,
        "{6}: This creature gets +2/+2 until end of turn. This ability costs {X} less to activate, where X is the number of differently named lands you control. Activate only once each turn.")
        .once_each_turn()
        .cost_change(cost_reduction)],
    ..oracle::ASTUTE_TERRAFORMER
};
