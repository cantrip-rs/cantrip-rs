use cantrip_api::*;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_mechanics::{history, pump, targets};
use cantrip_oracle as oracle;

/// "Target creature an opponent controls gets -1/-1 until end of turn. If a creature died this turn, that creature gets
/// -13/-13 until end of turn instead." (CR 700.4, 608.2c, 611.2a.) Morbid is no intervening if: the ability triggers and
/// resolves either way, and whether a creature died this turn is looked at as it resolves, the creature getting the one
/// modifier or the other, never both. Nothing happens if the target is no longer legal (CR 608.2b).
fn shrink(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    if history::creatures_died_this_turn(g, None) > 0 {
        pump(g, item.object, creature, -13, -13);
    } else {
        pump(g, item.object, creature, -1, -1);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        THIS_ENTERS,
        shrink,
        "Morbid \u{2014} When this creature enters, target creature an opponent controls gets -1/-1 until end of turn. If a creature died this turn, that creature gets -13/-13 until end of turn instead.",
    )
    .targets(&[targets::CREATURE_AN_OPPONENT_CONTROLS])],
    ..oracle::TRAGIC_BANSHEE
};
