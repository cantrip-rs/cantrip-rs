use cantrip_api::*;
use cantrip_mechanics::triggers::THIS_BECOMES_TAPPED;
use cantrip_mechanics::{effects, learn};
use cantrip_oracle as oracle;

/// "Noncreature spells you cast cost {1} less to cast as long as there are three or more Lesson cards in your graveyard"
/// (CR 601.2f, 118.7a): asked of the spell's own characteristics as it is cast, only its controller's, and only while the
/// controller's graveyard holds three Lesson cards; a generic reduction takes only the generic part of the cost.
fn noncreature_spells_cost_one_less(q: &dyn Query, source: ObjectId, player: PlayerId, costed: Costed, _: &[Target]) -> CostDelta {
    let Costed::Spell(spell) = costed else { return CostDelta::NONE };
    if player != q.controller(source) || q.chars(spell).is(CardType::Creature) {
        return CostDelta::NONE;
    }
    if learn::lesson_cards_in_graveyard(q, player) >= 3 { CostDelta::less(1) } else { CostDelta::NONE }
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(THIS_BECOMES_TAPPED, effects::loot, "Whenever Gran-Gran becomes tapped, draw a card, then discard a card.")],
    cost_changes: &[noncreature_spells_cost_one_less],
    ..oracle::GRAN_GRAN
};
