use cantrip_api::*;
use cantrip_mechanics::vivid;
use cantrip_oracle as oracle;

/// "Vivid - This spell costs {1} less to cast for each color among permanents you control" (CR 601.2f, 700.4): judged once, as the
/// total cost is determined, for the player casting it; generic mana only (CR 118.7a), and colorless is no color.
fn cheaper_per_color(q: &dyn Query, _: ObjectId, caster: PlayerId, _: Costed, _: &[Target]) -> CostDelta {
    CostDelta::less(vivid::count(q, caster))
}

/// Reach and trample are core keywords.
pub static CARD: CardDef = CardDef { own_cost_changes: &[cheaper_per_color], ..oracle::WILDVINE_PUMMELER };
