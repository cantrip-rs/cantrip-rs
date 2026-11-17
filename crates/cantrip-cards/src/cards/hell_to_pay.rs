use cantrip_api::*;
use cantrip_mechanics::{targets, tokens};
use cantrip_oracle as oracle;

/// "Hell to Pay deals X damage to target creature. Create a number of tapped Treasure tokens equal to the amount of excess damage
/// dealt to that creature this way." (CR 120.10, 608.2c): the excess is what the damage procedure found above lethal damage once
/// prevention, replacement, damage already marked and deathtouch were applied. With the creature an illegal target the spell does
/// not resolve (CR 608.2b); with no damage dealt to it there is no excess and no token. X is the value announced as it was cast (CR 107.3a).
fn damage_then_tapped_treasures_equal_to_excess(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let target = item.target(0);
    let Some(result) = g.damage_result(item.object, target, item.x as u32) else { return Ok(()) };
    g.end_sentence();
    if result.target == target {
        tokens::create_by_rule(g, item.controller, &tokens::TREASURE, result.excess, true)?;
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE], damage_then_tapped_treasures_equal_to_excess),
    ..oracle::HELL_TO_PAY
};

pub static EXTRA: &[&CardDef] = &[&tokens::TREASURE];
