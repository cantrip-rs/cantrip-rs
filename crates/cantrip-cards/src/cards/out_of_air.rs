use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "This spell costs {2} less to cast if it targets a creature spell" (CR 601.2c, 601.2f, 118.7a): judged once the target is announced,
/// as the total cost is determined; generic mana only. A spell on the stack has the characteristics of the card it is, so a creature
/// spell is one whose card is a creature.
fn less_if_it_targets_a_creature_spell(q: &dyn Query, _: ObjectId, _: PlayerId, _: Costed, targets: &[Target]) -> CostDelta {
    if targets.iter().filter_map(|t| t.object()).any(|spell| q.chars(spell).is(CardType::Creature)) {
        CostDelta::less(2)
    } else {
        CostDelta::NONE
    }
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::SPELL], effects::counter_first_target),
    own_cost_changes: &[less_if_it_targets_a_creature_spell],
    ..oracle::OUT_OF_AIR
};
