use cantrip_api::*;
use cantrip_oracle as oracle;
use cantrip_mechanics::targets;

/// "Reckless Rage deals 4 damage to target creature you don't control and 2 damage to target creature you control": one
/// sentence, so the damage is dealt at once (CR 120.3, 608.2c). A target that has become illegal is left out and the other
/// is still dealt its damage (CR 608.2b; the spell is countered only if both are illegal).
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let hits: Vec<(ObjectId, Target, i64)> = [(0, 4), (1, 2)]
        .into_iter()
        .filter(|&(slot, _)| g.target_legal(item, slot))
        .map(|(slot, amount)| (item.object, g.state().target(item, slot), amount))
        .collect();
    g.damage_together(&hits);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(
        &[
            targets::CREATURE_YOU_DONT_CONTROL.labelled("Creature you don't control to be dealt 4 damage"),
            targets::CREATURE_YOU_CONTROL.labelled("Creature you control to be dealt 2 damage"),
        ],
        effect,
    ),
    ..oracle::RECKLESS_RAGE
};
