use cantrip_api::*;
use cantrip_mechanics::triggers::THIS_ATTACKS;
use cantrip_mechanics::{destroy_target, effects, targets};
use cantrip_oracle as oracle;

const TAKE_DAMAGE: Question = Question::new("TakeDamage", "have Combustion Man deal damage to you equal to his power?", "Otherwise the permanent is destroyed", "Take the damage", "Let it be destroyed");

/// "Destroy target permanent unless its controller has Combustion Man deal damage to them equal to his power." As it resolves,
/// the target's controller (CR 608.2b: nothing happens if it is no longer a legal target) chooses; if they do, Combustion Man
/// deals damage to them equal to the power he has, or last had if he has left the battlefield (CR 608.2h, 113.7a), and the
/// permanent is not destroyed. Damage that is prevented or redirected is still the choice made.
fn destroy_unless_damaged(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(permanent) = targets::object(g, item, 0) else { return Ok(()) };
    let controller = g.controller(permanent);
    if effects::ask_may(g, controller, item.object, Purpose::Ask(&TAKE_DAMAGE))? {
        let power = i64::from(targets::power_or_last_known(g, item.object).max(0));
        g.damage(item.object, Target::Player(controller), power);
    } else {
        destroy_target(g, item, 0);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        THIS_ATTACKS,
        destroy_unless_damaged,
        "Whenever Combustion Man attacks, destroy target permanent unless its controller has Combustion Man deal damage to them equal to his power.",
    )
    .targets(&[targets::PERMANENT])],
    ..oracle::COMBUSTION_MAN
};
