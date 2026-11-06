use cantrip_api::*;
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

/// "Enchanted artifact's activated abilities cost {2} less to activate. This effect can't reduce the mana in that cost to less than one mana." (CR 604.1).
fn enchanted_artifacts_activated_abilities_cost_rule(
    q: &dyn Query,
    source: ObjectId,
    _: PlayerId,
    costed: Costed,
    _: &[Target],
) -> CostDelta {
    let asker = &Asker::new(source, q.controller(source));
    let Costed::Ability(permanent, ability, _) = costed else { return CostDelta::NONE };
    let def = q.activated_ability(ability);
    if def.is_some_and(|d| d.from == ZoneKind::Battlefield)
        && Some(asker.source).and_then(|o: ObjectId| q.state().obj(o).attached_to)
            == Some(permanent)
    {
        CostDelta::less_to_one(2)
    } else {
        CostDelta::NONE
    }
}

pub static CARD: CardDef = CardDef {
    enchant: Some(targets::ARTIFACT),
    cost_changes: &[enchanted_artifacts_activated_abilities_cost_rule],
    ..oracle::POWER_ARTIFACT
};
