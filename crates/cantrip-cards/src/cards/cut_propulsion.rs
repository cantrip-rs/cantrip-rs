use cantrip_api::*;
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

/// "Target creature deals damage to itself equal to its power. If that creature has flying, it deals twice that much
/// damage to itself instead." (CR 120.3, 608.2c): the creature is the source of the damage, so its own abilities
/// count. Its power and whether it has flying are read as the spell resolves; a creature with no power deals none (CR
/// 120.8). The target is judged on its own (CR 608.2b): once it is not a legal target nothing happens.
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    // The target is legal here (an illegal sole target means the spell never resolves, CR 608.2b), so there is one creature.
    for creature in targets::object(g, item, 0).into_iter() {
        let chars = g.chars(creature);
        // In i32 so doubling cannot wrap; a power of 0 or less deals no damage, so a negative never reaches the u16 conversion.
        let power = i32::from(chars.power.unwrap_or(0));
        let amount = if chars.has(Keyword::Flying) { power * 2 } else { power };
        if amount > 0 {
            g.damage(creature, Target::Object(creature), i64::from(u16::try_from(amount).unwrap_or(u16::MAX)));
        }
    }
    Ok(())
}

pub static CARD: CardDef = CardDef { spell: spell(&[targets::CREATURE], effect), ..oracle::CUT_PROPULSION };
