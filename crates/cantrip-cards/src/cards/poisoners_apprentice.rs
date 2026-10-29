use cantrip_api::*;
use cantrip_mechanics::history::gained_life_this_turn;
use cantrip_mechanics::{effects, targets, triggers::THIS_ENTERS};
use cantrip_oracle as oracle;

/// "Target creature an opponent controls gets -4/-4 until end of turn if you gained life this turn": it always triggers and
/// has its target chosen, and whether life was gained is asked as it resolves (CR 608.2c), not as it triggers.
fn shrink_if_life_was_gained(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if gained_life_this_turn(&*g, item.controller) {
        effects::target_creature_gets::<-4, -4>(g, item)?;
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        THIS_ENTERS,
        shrink_if_life_was_gained,
        "Infusion \u{2014} When this creature enters, target creature an opponent controls gets -4/-4 until end of turn if you gained life this turn.",
    )
    .targets(&[targets::CREATURE_AN_OPPONENT_CONTROLS])],
    ..oracle::POISONERS_APPRENTICE
};
