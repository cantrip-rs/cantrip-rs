use cantrip_api::*;
use cantrip_mechanics::{damage_target, exile_and_return, greatest_power, targets};
use cantrip_oracle as oracle;

/// "Exile target creature you control, then return it to the battlefield under its owner's control" (CR 608.2c, 400.7): it comes
/// back as a new object; a token exiled this way does not return (CR 111.7).
fn blink(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    exile_and_return(g, &[creature])
}

/// "Go Ninja Go deals damage equal to the greatest power among creatures you control to target creature an opponent controls": the
/// power is read as this mode is performed (CR 608.2h), after the first mode if both were chosen, so a creature that came back
/// from it counts as it is now; none (or none above 0) deals no damage (CR 120.8).
fn damage(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let greatest = greatest_power::among_creatures_you_control(g, item.controller);
    damage_target(g, item, 0, greatest);
    Ok(())
}

/// "Choose one or both" (CR 700.2): the modes are carried out in the order printed (CR 700.2a).
static MODAL: ModalDef = modes(&[
    mode(blink, "Exile target creature you control, then return it to the battlefield under its owner's control.")
        .targets(&[targets::CREATURE_YOU_CONTROL]),
    mode(damage, "Go Ninja Go deals damage equal to the greatest power among creatures you control to target creature an opponent controls.")
        .targets(&[targets::CREATURE_YOU_DONT_CONTROL]),
])
.choose(1, 2);

pub static CARD: CardDef = CardDef { spell: Some(SpellDef::MODAL), modal: Some(&MODAL), ..oracle::GO_NINJA_GO };
