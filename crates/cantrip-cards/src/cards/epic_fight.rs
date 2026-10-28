use cantrip_api::*;
use cantrip_mechanics::{double, fight, targets};
use cantrip_oracle as oracle;

/// "Double target creature's power and toughness until end of turn." (CR 701.10a-b): the power and toughness it has as the
/// spell resolves, read once and fixed (CR 608.2h); a creature that is no longer a legal target is left alone (CR 608.2b).
fn double_power_and_toughness(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    double::power_and_toughness(g, item.object, creature);
    Ok(())
}

/// "Target creature you control fights target creature an opponent controls." (CR 701.14a-b): neither deals damage when either
/// target is no longer legal.
fn fight_targets(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    fight::targets_fight(g, item, 0, 1);
    Ok(())
}

/// "Choose one or both" (CR 700.2): the modes are carried out in the order printed (CR 700.2d), so with both, a creature that is
/// doubled and then fights deals damage by its doubled power.
static MODAL: ModalDef = modes(&[
    mode(double_power_and_toughness, "Double target creature's power and toughness until end of turn.").targets(&[targets::CREATURE]),
    mode(fight_targets, "Target creature you control fights target creature an opponent controls.")
        .targets(&[targets::CREATURE_YOU_CONTROL, targets::CREATURE_AN_OPPONENT_CONTROLS]),
])
.choose(1, 2);

pub static CARD: CardDef = CardDef { spell: Some(SpellDef::MODAL), modal: Some(&MODAL), ..oracle::EPIC_FIGHT };
