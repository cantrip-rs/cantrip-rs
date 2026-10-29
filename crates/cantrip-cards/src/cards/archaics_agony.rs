use cantrip_api::*;
use cantrip_mechanics::{library, sunburst, targets};
use cantrip_oracle as oracle;

/// Converge: "Archaic's Agony deals X damage to target creature, where X is the number of colors of mana spent to cast this
/// spell. Exile cards from the top of your library equal to the excess damage dealt to that creature this way. You may play
/// those cards until the end of your next turn." X is zero for a copy, and no damage is dealt then (CR 120.8). The excess is
/// what the damage procedure found above lethal damage after prevention, replacement, damage already marked and deathtouch
/// (CR 120.10), and counts only damage dealt to that creature.
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let target = item.target(0);
    let x = sunburst::colors_spent(g, item.object) as u16;
    if let Some(result) = g.damage_result(item.object, target, x)
        && result.target == target
        && result.excess > 0
    {
        library::exile_top_play_until_end_of_next_turn(g, item.controller, result.excess as usize);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef { spell: spell(&[targets::CREATURE], effect), ..oracle::ARCHAICS_AGONY };
