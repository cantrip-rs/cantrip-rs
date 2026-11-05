use cantrip_api::*;
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

/// "Target creature deals damage equal to its power to itself. If that creature is attacking, ~ deals 2 damage to that
/// creature's controller." (CR 120.3, 608.2c): the creature is the source of the damage to itself, so its own abilities
/// count; a creature with no power deals none (CR 120.8). State-based actions wait for the spell to finish (CR 704.3), so
/// the creature is still there, attacking or not, for the second sentence; Wisecrack itself is the source of the 2.
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    // A power of 0 or less deals none (the core deals nothing for 0), so only a positive power is dealt.
    let power = g.chars(creature).power.unwrap_or_default();
    g.damage(creature, Target::Object(creature), i64::from(power.max(0)));
    if g.is_attacking(creature) {
        let controller = g.controller(creature);
        g.damage(item.object, Target::Player(controller), 2);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef { spell: spell(&[targets::CREATURE], effect), ..oracle::WISECRACK };
