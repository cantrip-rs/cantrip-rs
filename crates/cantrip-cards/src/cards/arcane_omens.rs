use cantrip_api::*;
use cantrip_mechanics::{discard_chosen, sunburst, targets};
use cantrip_oracle as oracle;

/// Converge: "target player discards X cards, where X is the number of colors of mana spent to cast this spell" (CR 701.9):
/// the player chooses which. A copy was not cast, so X is zero (`sunburst::colors_spent`).
fn discard_x(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(player) = targets::player(g, item, 0) else { return Ok(()) };
    let x = sunburst::colors_spent(g, item.object) as u32;
    discard_chosen(g, player, item.object, x)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::PLAYER], discard_x),
    ..oracle::ARCANE_OMENS
};
