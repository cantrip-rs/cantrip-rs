use cantrip_api::*;
use cantrip_mechanics::{effects, pump, targets};
use cantrip_oracle as oracle;

/// "Target creature gets -4/-0 until end of turn. Draw a card." A target no longer legal means the spell does not resolve
/// at all and draws nothing (CR 608.2b).
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    pump(g, item.object, creature, -4, 0);
    g.end_sentence();
    effects::draw(g, item, 1)
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE], effect),
    own_cost_changes: &[targets::costs_less_if_it_targets_an_attacking_creature::<2>],
    ..oracle::DEPOWER
};
