use cantrip_api::*;
use cantrip_mechanics::{attach, targets};
use cantrip_oracle as oracle;

/// "Judgment Bolt deals 5 damage to target creature and X damage to that creature's controller, where X is the number of
/// Equipment you control." (CR 120.2, 608.2b, 608.2h): one instruction, so both are dealt at once. X is counted once, as the
/// spell resolves (the ruling), and the controller is the creature's as it resolves. With the creature an illegal target the
/// spell does nothing (CR 608.2b).
fn five_to_the_creature_and_x_to_its_controller(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    let x = attach::equipment_controlled(g, item.controller).len() as u16;
    let controller = g.controller(creature);
    g.damage_together(&[(item.object, Target::Object(creature), 5), (item.object, Target::Player(controller), x)]);
    Ok(())
}

pub static CARD: CardDef = CardDef { spell: spell(&[targets::CREATURE], five_to_the_creature_and_x_to_its_controller), ..oracle::JUDGMENT_BOLT };
