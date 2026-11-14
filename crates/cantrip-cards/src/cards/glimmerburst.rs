use cantrip_api::*;
use cantrip_mechanics::{effects, tokens};
use cantrip_oracle as oracle;

/// "Draw two cards. Create a 1/1 white Glimmer enchantment creature token": two sentences, so the token is made after the
/// draw has happened (CR 608.2c).
fn draw_two_then_make_a_glimmer(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::draw(g, item, 2)?;
    g.end_sentence();
    tokens::create(g, item.controller, &tokens::GLIMMER, 1)?;
    Ok(())
}

pub static CARD: CardDef = CardDef { spell: spell(&[], draw_two_then_make_a_glimmer), ..oracle::GLIMMERBURST };

pub static EXTRA: &[&CardDef] = &[&tokens::GLIMMER];
