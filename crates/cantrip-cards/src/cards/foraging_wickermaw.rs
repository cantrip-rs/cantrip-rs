use cantrip_api::*;
use cantrip_mechanics::{effects, triggers::THIS_ENTERS};
use cantrip_oracle as oracle;

/// "Add one mana of any color. This creature becomes that color until end of turn" (CR 605.3a, 106.5, 105.3): the color named
/// as the ability was activated is the mana's and the creature's, which is exactly that color and no other (CR 613.1e); a
/// color-changing effect from before is replaced at its own timestamp.
fn add_mana_and_become_that_color(g: &mut dyn Game, asker: &Asker, color: Option<Color>) -> Result<(), Suspend> {
    let color = color.unwrap_or(Color::White);
    g.add_mana(asker.controller, color.mana(), 1);
    let words = g.words(asker.source);
    g.add_effect(asker.source, asker.source, EffectKind::SetColors(words.colors(ColorSet::of(color))), Duration::EndOfTurn);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(THIS_ENTERS, |g, item| effects::surveil(g, item, 1), "When this creature enters, surveil 1."),
        mana_ability(
            cost("{1}"),
            add_any_color(1).by(add_mana_and_become_that_color),
            "{1}: Add one mana of any color. This creature becomes that color until end of turn. Activate only once each turn.",
        )
        .once_each_turn(),
    ],
    ..oracle::FORAGING_WICKERMAW
};
