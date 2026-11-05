use cantrip_api::*;
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

/// "Razor Rings deals 4 damage to target attacking or blocking creature. You gain life equal to the excess damage dealt this way."
/// The damage procedure determines the actual excess after damage already marked, replacement and prevention (CR 120.10); none
/// dealt to that creature, or no excess, gains nothing (CR 119.3).
fn damage_and_gain_life_equal_to_excess(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let target = item.target(0);
    if let Some(result) = g.damage_result(item.object, target, 4)
        && result.target == target
    {
        g.gain_life(item.controller, result.excess);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::ATTACKING_OR_BLOCKING_CREATURE], damage_and_gain_life_equal_to_excess),
    ..oracle::RAZOR_RINGS
};
