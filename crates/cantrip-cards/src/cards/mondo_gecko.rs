use cantrip_api::*;
use cantrip_mechanics::triggers::DEALS_COMBAT_DAMAGE_TO_PLAYER;
use cantrip_mechanics::choose_color;
use cantrip_oracle as oracle;

/// "Until end of turn, Mondo Gecko becomes the color of your choice and gains hexproof from that color." The choice is made
/// as the ability resolves (CR 608.2d); becoming a color replaces its colors with that one (CR 105.3, layer 5) and the
/// hexproof is granted for that color for good, even if a later effect changes Mondo Gecko's color again (CR 702.11d, 611.2a).
fn become_a_color(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let color = choose_color(g, item.controller, item.object)?;
    let words = g.words(item.object);
    instruction(g, |g| {
        g.add_effect(item.object, item.object, EffectKind::SetColors(words.colors(ColorSet::of(color))), Duration::EndOfTurn);
        g.add_effect(item.object, item.object, EffectKind::HexproofFrom(HexproofFrom::color(color)), Duration::EndOfTurn);
    });
    Ok(())
}

/// "Draw a card for each color among permanents you control." (CR 105.2) A multicolored permanent counts once for each of its
/// colors, each color once however many permanents have it, read as the ability resolves.
fn draw_for_colors(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let you = item.controller;
    let permanents = g.permanents_of(you);
    let colors = Color::ALL.iter().filter(|&&color| permanents.iter().any(|&id| g.chars(id).colors.has(color))).count();
    g.draw(you, colors as u32);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(
            cost("{1}").and(discard_cards(1)),
            become_a_color,
            "{1}, Discard a card: Until end of turn, Mondo Gecko becomes the color of your choice and gains hexproof from that color.",
        ),
        triggered(
            DEALS_COMBAT_DAMAGE_TO_PLAYER,
            draw_for_colors,
            "Whenever Mondo Gecko deals combat damage to a player, draw a card for each color among permanents you control.",
        ),
    ],
    ..oracle::MONDO_GECKO
};
