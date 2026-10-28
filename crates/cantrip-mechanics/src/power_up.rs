//! Power-up (CR 702.193). "Power-up — [Cost]: [Effect]" means "[Cost]: [Effect]. If this permanent entered this turn,
//! this ability's cost is reduced by this permanent's mana cost. Activate this ability only once." (702.193a). Generic
//! mana in the permanent's mana cost reduces generic mana in the ability's cost; colored and colorless mana reduce mana
//! of the same type, and any excess reduces that much generic mana (702.193b, 118.7).
//!
//! Printed on Captain Marvel, Earth's Protector, Aerial Doombot, Brave Brawler, Serpent Specialist, Viv Vision, Teen
//! Synthezoid, Thanos, the Mad Titan and the rest of Marvel Super Heroes. A card lists `power_up::ability(cost, effect,
//! text)` among its `abilities`.
//!
//! A power-up permanent whose mana cost has a hybrid symbol (She-Hulk, Attorney-at-Law; Brawn, Amadeus Cho; Abomination,
//! Terrifying Titan) reduces by that symbol as the player splits it (CR 118.7e, the core's `announce_reduction`).

use cantrip_api::*;

use crate::history::entered_this_turn;

/// "If this permanent entered this turn, this ability's cost is reduced by this permanent's mana cost" (CR 702.193a-b):
/// the mana cost it has now, a copy's being the one it copies (CR 707.2). An object that left and came back entered
/// anew (CR 400.7).
pub fn reduction(q: &dyn Query, asker: &Asker, _: Costed, _: &[Target]) -> CostDelta {
    let permanent = asker.source;
    if !entered_this_turn(q.history(), permanent) {
        return CostDelta::NONE;
    }
    CostDelta::less_mana(q.mana_cost(permanent))
}

/// A power-up ability: `cost` reduced by the permanent's mana cost on the turn it entered, and activated only once
/// (CR 702.193a).
pub const fn ability(cost: Cost, effect: Effect, text: &'static str) -> AbilityDef {
    cantrip_api::ability(cost, effect, text).cost_change(reduction).power_up()
}
