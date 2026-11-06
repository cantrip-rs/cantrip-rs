use cantrip_api::*;
use cantrip_oracle as oracle;

/// Draw a card, then you lose life equal to the number of cards in your hand.
fn draw_then_lose_life(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let player = item.controller;
    g.draw(player, 1);
    let cards = g.state().zone(Zone::hand(player)).len() as i32;
    if cards > 0 {
        g.lose_life(player,(cards) as i32);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    enters: Some(cantrip_mechanics::lands::enters_tapped_unless_you_control::<{ cantrip_mechanics::lands::sub(Subtype::Swamp) }>),
    abilities: &[
        mana_ability(TAP, add(Mana::B), "{T}: Add {B}."),
        ability(cost("{1}{B}{B}, {T}"), draw_then_lose_life, "{1}{B}{B}, {T}: Draw a card, then you lose life equal to the number of cards in your hand."),
    ],
    ..oracle::CASTLE_LOCTHWAIN
};
