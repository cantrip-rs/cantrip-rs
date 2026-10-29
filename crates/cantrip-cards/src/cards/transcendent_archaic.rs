use cantrip_api::*;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_mechanics::{discard_chosen, effects, sunburst};
use cantrip_oracle as oracle;

const ASK: Question = Question::new(
    "Draw",
    "draw a card for each color of mana spent to cast this creature?",
    "If you draw one or more cards this way, you then discard two cards",
    "Draw cards",
    "Don't draw",
);

/// Converge: "you may draw X cards, where X is the number of colors of mana spent to cast this spell. If you draw one or more
/// cards this way, discard two cards" (CR 701.9, 121.1, 608.2c). X is read from the creature itself (CR 601.2h), so it still
/// answers if the creature has left. The discard follows only if the draw really drew a card (CR 121.2, 121.4), read from the
/// draws this resolution recorded: a library that gave none, or a draw of zero, leaves nothing to discard.
fn draw_then_discard(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if !effects::ask_may(g, item.controller, item.object, Purpose::Ask(&ASK))? {
        return Ok(());
    }
    let before = g.history().head() as usize;
    g.draw(item.controller, sunburst::colors_spent(g, item.object) as u32);
    let drew = g.history().all()[before..].iter().any(|r| matches!(r.event, Event::Drew { player, .. } if player == item.controller));
    if drew {
        discard_chosen(g, item.controller, item.object, 2)?;
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        THIS_ENTERS,
        draw_then_discard,
        "Converge \u{2014} When this creature enters, you may draw X cards, where X is the number of colors of mana spent to cast this spell. If you draw one or more cards this way, discard two cards.",
    )],
    ..oracle::TRANSCENDENT_ARCHAIC
};
