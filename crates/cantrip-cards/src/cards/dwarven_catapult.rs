use cantrip_api::*;
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

/// Divide without narrowing the creature count; the quotient is bounded by X.
pub(crate) fn damage_share(x: u32, creatures: usize) -> u32 {
    (u64::from(x) / creatures as u64) as u32
}

/// Divide X by the number of creatures the opponent controls at resolution (CR 608.2h).
fn damage_creatures(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(opponent) = targets::player(g, item, 0) else { return Ok(()) };
    let creatures: Vec<_> = g.permanents_of(opponent).into_iter()
        .filter(|&id| g.chars(id).is(CardType::Creature))
        .collect();
    if creatures.is_empty() {
        return Ok(());
    }
    let damage = damage_share(item.x, creatures.len());
    let hits: Vec<_> = creatures.into_iter().map(|id| (item.object, Target::Object(id), u32::from(damage))).collect();
    g.damage_together(&hits);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::OPPONENT], damage_creatures),
    ..oracle::DWARVEN_CATAPULT
};
