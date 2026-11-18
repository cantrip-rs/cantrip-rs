use cantrip_api::*;
use cantrip_mechanics::{permanents_with_subtype_you_control, subtype_cards_in_graveyard};
use cantrip_oracle as oracle;

/// "This spell costs {1} less to cast for each Cave you control and each Cave card in your graveyard" (CR 601.2f, 118.7a): both counts taken as the
/// cost is determined - the permanents as the layers have made them Caves, the cards as they are in the graveyard - and only the caster's, so another
/// player's Caves count in neither place. The reduction never takes off a colored symbol (CR 118.7a). Only the cost of casting it; its mana value stays 8.
fn less_per_cave_you_control_and_in_your_graveyard(q: &dyn Query, _: ObjectId, player: PlayerId, _: Costed, _: &[Target]) -> CostDelta {
    let controlled = permanents_with_subtype_you_control(q, player, Subtype::Cave);
    let cards = subtype_cards_in_graveyard(q, player, Subtype::Cave);
    CostDelta::less((controlled + cards) as u32)
}

/// Lifelink is in the content JSON.
pub static CARD: CardDef = CardDef { own_cost_changes: &[less_per_cave_you_control_and_in_your_graveyard], ..oracle::GARGANTUAN_LEECH };
