use cantrip_api::*;
use cantrip_oracle as oracle;

/// "Spells you cast cost {1} less to cast." (CR 604.1).
fn spells_cast_cost_1_less_rule(
    q: &dyn Query,
    source: ObjectId,
    player: PlayerId,
    costed: Costed,
    _: &[Target],
) -> CostDelta {
    let asker = &Asker::new(source, q.controller(source));
    let Costed::Spell(_) = costed else { return CostDelta::NONE };
    if player == asker.controller { CostDelta::less(1) } else { CostDelta::NONE }
}

pub static CARD: CardDef = CardDef {
    cost_changes: &[spells_cast_cost_1_less_rule],
    ..oracle::STONE_CALENDAR
};
