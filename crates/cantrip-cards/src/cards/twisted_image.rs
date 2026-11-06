use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "Switch target creature's power and toughness until end of turn" (CR 613.4d, 611.2c): a layer 7d effect fixed to the
/// target as the spell resolves, so whatever sets or modifies its power and toughness, a pump cast afterwards included,
/// is applied before the switch. Then "draw a card". A spell whose target is illegal does nothing (CR 608.2b).
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    effects::switch_power_toughness_this_turn(g, item.object, creature);
    g.draw(item.controller, 1);
    Ok(())
}

pub static CARD: CardDef = CardDef { spell: spell(&[targets::CREATURE], effect), ..oracle::TWISTED_IMAGE };
