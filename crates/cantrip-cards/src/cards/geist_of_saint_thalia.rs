use cantrip_api::*;
use cantrip_mechanics::cost_reduction::noncreature_spells_you_cast;
use cantrip_oracle as oracle;

/// "Noncreature spells you cast cost {1} less to cast" (CR 601.2f, 118.7c).
fn noncreature_spells_cost_less(q: &dyn Query, source: ObjectId, player: PlayerId, costed: Costed, _: &[Target]) -> CostDelta {
    noncreature_spells_you_cast(q, source, player, costed, 1)
}

pub static CARD: CardDef = CardDef { cost_changes: &[noncreature_spells_cost_less], ..oracle::GEIST_OF_SAINT_THALIA };
