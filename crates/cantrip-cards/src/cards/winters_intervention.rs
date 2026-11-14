use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "Winter's Intervention deals 2 damage to target creature. You gain 2 life." (CR 120.3, 119.1)
pub static CARD: CardDef = CardDef { spell: spell(&[targets::CREATURE], |g, item| effects::damage_target_and_gain_life(g, item, 2, 2)), ..oracle::WINTERS_INTERVENTION };
