use cantrip_api::*;
use cantrip_mechanics::{targets, tokens};
use cantrip_oracle as oracle;

/// "Goblin Negotiation deals X damage to target creature. Create a number of 1/1 red Goblin creature tokens equal to the amount
/// of excess damage dealt to that creature this way." (CR 120.10, 608.2c): the excess is what the damage procedure found above
/// lethal damage once prevention, replacement, damage already marked and deathtouch were applied, counted only for this damage
/// to this creature. With the creature an illegal target the spell does not resolve (CR 608.2b); with no damage dealt to
/// it (prevented, or dealt elsewhere instead) there is no excess and no token. X is zero for a copy (CR 107.3), so nothing happens.
fn damage_then_goblins_equal_to_excess(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let target = item.target(0);
    let Some(result) = g.damage_result(item.object, target, item.x as u32) else { return Ok(()) };
    g.end_sentence();
    if result.target == target {
        tokens::create(g, item.controller, &tokens::GOBLIN, result.excess)?;
    }
    Ok(())
}

pub static CARD: CardDef = CardDef { spell: spell(&[targets::CREATURE], damage_then_goblins_equal_to_excess), ..oracle::GOBLIN_NEGOTIATION };

pub static EXTRA: &[&CardDef] = &[&tokens::GOBLIN];
