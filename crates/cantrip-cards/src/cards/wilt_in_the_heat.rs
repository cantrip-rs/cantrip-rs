use cantrip_api::*;
use cantrip_mechanics::history::cards_left_graveyard_this_turn;
use cantrip_mechanics::replacements::{EXILE_INSTEAD_OF_DYING, exile_if_it_would_die};
use cantrip_mechanics::{damage_target, targets};
use cantrip_oracle as oracle;

/// "This spell costs {2} less to cast if a card left your graveyard this turn" (CR 601.2f, 700.4): judged once, as the total cost
/// is determined, from the event history, and generic mana only (CR 118.7a).
fn less_if_a_card_left_your_graveyard(q: &dyn Query, _: ObjectId, caster: PlayerId, _: Costed, _: &[Target]) -> CostDelta {
    if cards_left_graveyard_this_turn(q, caster) > 0 { CostDelta::less(2) } else { CostDelta::NONE }
}

/// "Wilt in the Heat deals 5 damage to target creature. If that creature would die this turn, exile it instead": the replacement
/// is the spell's own effect on that creature for the turn (CR 611.2c, 614.1a), as Combustion Technique's is.
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    exile_if_it_would_die(g, item.object, 0, creature);
    damage_target(g, item, 0, 5);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE], effect),
    granted_replacements: &[EXILE_INSTEAD_OF_DYING],
    own_cost_changes: &[less_if_a_card_left_your_graveyard],
    ..oracle::WILT_IN_THE_HEAT
};
