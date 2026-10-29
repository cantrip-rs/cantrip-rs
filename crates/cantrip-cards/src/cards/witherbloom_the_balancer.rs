use cantrip_api::*;
use cantrip_mechanics::affinity;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    own_cost_changes: &[affinity::for_creatures],
    cost_changes: &[affinity::instants_and_sorceries_you_cast_have_affinity_for_creatures],
    ..oracle::WITHERBLOOM_THE_BALANCER
};
