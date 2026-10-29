use cantrip_api::*;
use cantrip_mechanics::{effects, targets, triggers};
use cantrip_oracle as oracle;

/// The landfall trigger adds one mana of either named color (CR 106.1b); the choice is made as the trigger resolves (CR 608.2d).
fn add_red_or_green(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let choices = ColorSet::of(Color::Red).with(Color::Green);
    let color = cantrip_mechanics::choose_colors(g, item.controller, item.object, choices, 1)?[0];
    g.add_mana(item.controller, color.mana(), 1);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(triggers::LANDFALL, add_red_or_green, "Landfall — Whenever a land you control enters, add {R} or {G}."),
        ability(cost("{3}").and(sacrifice(1, targets::filters::land)), |g, item| effects::draw(g, item, 1), "{3}, Sacrifice a land: Draw a card."),
    ],
    ..oracle::DAL_REL_AUGMENT_ORPHAN
};
