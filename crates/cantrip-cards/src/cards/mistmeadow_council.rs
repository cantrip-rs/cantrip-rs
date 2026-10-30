use cantrip_api::*;
use cantrip_mechanics::{effects, triggers};
use cantrip_oracle as oracle;

/// "This spell costs {1} less to cast if you control a Kithkin" (CR 601.2f): judged once, as the total cost is determined, and
/// generic mana only (CR 118.7a); the Kithkin type is read through the words of this spell's text (CR 612).
fn less_with_a_kithkin(q: &dyn Query, spell: ObjectId, player: PlayerId, _: Costed, _: &[Target]) -> CostDelta {
    let kithkin = q
        .state()
        .battlefield()
        .iter()
        .any(|&id| q.controller(id) == player && q.chars(id).has_subtype(q.words(spell).subtype(Subtype::Kithkin)));
    if kithkin { CostDelta::less(1) } else { CostDelta::NONE }
}

pub static CARD: CardDef = CardDef {
    own_cost_changes: &[less_with_a_kithkin],
    abilities: &[triggered(triggers::THIS_ENTERS, |g, item| effects::draw(g, item, 1), "When this creature enters, draw a card.")],
    ..oracle::MISTMEADOW_COUNCIL
};
