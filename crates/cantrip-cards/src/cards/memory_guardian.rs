use cantrip_api::*;
use cantrip_mechanics::affinity;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef { own_cost_changes: &[affinity::for_artifacts], ..oracle::MEMORY_GUARDIAN };
