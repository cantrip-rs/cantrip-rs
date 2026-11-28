use cantrip_api::*;
use cantrip_mechanics::{discard_chosen, effects, triggers};
use cantrip_oracle as oracle;

const ASK: Question = Question::new(
    "Draw",
    "draw cards equal to the mana spent to cast that spell?",
    "If you do, you then discard two cards",
    "Draw cards",
    "Don't draw",
);

/// "You may draw X cards, where X is the amount of mana spent to cast that spell. If you do, discard two cards." (CR 601.2h,
/// 121.1, 701.9, 608.2c): all the mana spent counts, of any type and on any part of the cost, read from the spell's own object
/// so it still answers once the spell has been countered or has left the stack (CR 608.2h); a spell cast without paying its
/// mana cost had none spent. Drawing zero cards is not a draw at all (CR 121.2; compare CR 701.22b, 701.25c) and so the choice is not offered, and "if you do" is the draw that was
/// chosen: the discard follows the draw however many cards the library gave (CR 121.4).
fn draw_then_discard(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Target::Object(spell) = item.subject else { return Ok(()) };
    let x = g.obj(spell).spent.total();
    if x == 0 || !effects::ask_may(g, item.controller, item.object, Purpose::Ask(&ASK))? {
        return Ok(());
    }
    g.draw(item.controller, x as u32);
    discard_chosen(g, item.controller, item.object, 2)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::NONCREATURE_SPELL_YOU_CAST_ABOUT_IT,
        draw_then_discard,
        "Whenever you cast a noncreature spell, you may draw X cards, where X is the amount of mana spent to cast that spell. If you do, discard two cards.",
    )],
    ..oracle::UNCOVER_THE_MOON_LETTERS
};
