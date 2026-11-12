use cantrip_api::*;
use cantrip_mechanics::cost_reduction;
use cantrip_oracle as oracle;

/// "Artifact spells you cast cost {1} less to cast" (CR 601.2f): only its controller's spells, judged on the spell's own
/// characteristics as it is cast; a generic reduction takes only the generic part (CR 118.7a). Abilities are not spells.
fn artifact_spells_cost_one_less(q: &dyn Query, source: ObjectId, player: PlayerId, costed: Costed, _: &[Target]) -> CostDelta {
    cost_reduction::spells_you_cast(q, source, player, costed, CardType::Artifact, 1)
}

pub static CARD: CardDef = CardDef { cost_changes: &[artifact_spells_cost_one_less], ..oracle::VOYAGER_QUICKWELDER };
