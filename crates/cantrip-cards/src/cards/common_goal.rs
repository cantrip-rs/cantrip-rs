use cantrip_api::*;
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

/// "One or two target creatures you control each deal damage equal to their power to target creature an opponent
/// controls." The powers are read as this resolves and the damage is dealt together (CR 120.2, 120.3, 120.4b, 608.2b).
fn deal_damage(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let recipient_slot = item.target_count as usize - 1;
    let Some(recipient) = targets::object(g, item, recipient_slot) else { return Ok(()) };
    let hits: Vec<(ObjectId, Target, u16)> = (0..recipient_slot)
        .filter_map(|slot| {
            let source = targets::object(g, item, slot)?;
            let power = g.chars(source).power.unwrap_or(0).max(0) as u16;
            Some((source, Target::Object(recipient), power))
        })
        .collect();
    g.damage_together(&hits);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(
        &[
            targets::CREATURE_YOU_CONTROL.between(1, 2),
            targets::CREATURE_AN_OPPONENT_CONTROLS,
        ],
        deal_damage,
    ),
    ..oracle::COMMON_GOAL
};
