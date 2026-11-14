use cantrip_api::*;
use cantrip_mechanics::{choose, effects, targets, triggers};
use cantrip_oracle as oracle;

/// Choose a nonempty set of colors, which lasts indefinitely (CR 105.2, 611.2a).
fn choose_colors(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if effects::ask_may(g, item.controller, item.object, Purpose::Optional)? {
        // These are color names, not words in Shyft’s rules text.
        let options = (0..31).map(|n| Label::WordedCard(item.ability.card, n, Words::NONE)).collect();
        let chosen = choose::one_of(g, item.controller, item.object, options)? + 1;
        let colors = ColorSet(chosen);
        if targets::permanent(g, item.object) {
            g.add_effect(item.object, item.object, EffectKind::SetColors(colors), Duration::Indefinite);
        }
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(triggers::YOUR_UPKEEP, choose_colors,
        "At the beginning of your upkeep, you may have this creature become the color or colors of your choice.")],
    labels: &[
        "White",
        "Blue",
        "White, Blue",
        "Black",
        "White, Black",
        "Blue, Black",
        "White, Blue, Black",
        "Red",
        "White, Red",
        "Blue, Red",
        "White, Blue, Red",
        "Black, Red",
        "White, Black, Red",
        "Blue, Black, Red",
        "White, Blue, Black, Red",
        "Green",
        "White, Green",
        "Blue, Green",
        "White, Blue, Green",
        "Black, Green",
        "White, Black, Green",
        "Blue, Black, Green",
        "White, Blue, Black, Green",
        "Red, Green",
        "White, Red, Green",
        "Blue, Red, Green",
        "White, Blue, Red, Green",
        "Black, Red, Green",
        "White, Black, Red, Green",
        "Blue, Black, Red, Green",
        "White, Blue, Black, Red, Green",
    ],
    ..oracle::SHYFT
};
