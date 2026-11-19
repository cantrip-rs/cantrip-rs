use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "Destroy target enchantment. If a permanent you controlled or a token was destroyed this way, draw a card" (CR 701.8a, 608.2c):
/// whose it was and whether it is a token are read before it goes (CR 608.2h), and the card is drawn only when it was actually
/// destroyed, not when it is indestructible or a regeneration shield saved it.
fn destroy_enchantment_then_draw_if_yours_or_a_token(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(enchantment) = targets::object(g, item, 0) else { return Ok(()) };
    let yours_or_a_token = g.controller(enchantment) == item.controller || g.state().obj(enchantment).token;
    let destroyed = g.destroy_all(&[enchantment]);
    if yours_or_a_token && !destroyed.is_empty() {
        effects::draw(g, item, 1)?;
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::ENCHANTMENT], destroy_enchantment_then_draw_if_yours_or_a_token),
    ..oracle::BREAK_THE_SPELL
};
