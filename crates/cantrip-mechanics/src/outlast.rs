//! Outlast (CR 702.107a), an activated ability: "Outlast [cost]" means "[Cost], {T}: Put a +1/+1 counter on this creature.
//! Activate only as a sorcery." Disowned Ancestor and Salt Road Patrol print it. The {T} in the cost is why it can't be
//! activated before the creature has been under its controller's control since the turn began (CR 302.6).
//!
//! A card with outlast lists `ability(cost, text)` among its `abilities`, `cost` the symbols as printed.

use crate::effects::plus_one_counter_on_this;
use cantrip_api::*;

/// `symbols` is the outlast cost's mana, as printed ("{1}{B}"); `text` the ability's full text.
pub const fn ability(symbols: &str, text: &'static str) -> AbilityDef {
    cantrip_api::ability(Cost { tap: true, ..of_mana(mana(symbols)) }, plus_one_counter_on_this, text).sorcery_speed()
}
