use cantrip_api::*;
use cantrip_mechanics::vivid;
use cantrip_oracle as oracle;

/// Reach and trample are core keywords.
pub static CARD: CardDef = CardDef { own_cost_changes: &[vivid::cheaper_per_color], ..oracle::WILDVINE_PUMMELER };
