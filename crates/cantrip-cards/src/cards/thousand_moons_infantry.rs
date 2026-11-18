use cantrip_api::*;
use cantrip_mechanics::targets::filters;
use cantrip_oracle as oracle;

/// "Untap this creature during each other player's untap step" (CR 502.3, 611.3a): the rule is true of this creature only.
pub static CARD: CardDef = CardDef { untap_in_others_untap_steps: &[filters::this_permanent], ..oracle::THOUSAND_MOONS_INFANTRY };
