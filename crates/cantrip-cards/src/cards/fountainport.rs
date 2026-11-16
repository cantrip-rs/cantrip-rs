use cantrip_api::*;
use cantrip_mechanics::targets;
use cantrip_mechanics::tokens::{self, FISH};
use cantrip_oracle as oracle;

/// "Draw a card."
fn draw_a_card(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.draw(item.controller, 1);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        mana_ability(TAP, add(Mana::C), "{T}: Add {C}."),
        ability(cost("{2}, {T}").and(sacrifice(1, targets::token_you_control)), draw_a_card, "{2}, {T}, Sacrifice a token: Draw a card."),
        ability(cost("{3}, {T}").and(life(1)), tokens::create_a_fish, "{3}, {T}, Pay 1 life: Create a 1/1 blue Fish creature token."),
        ability(cost("{4}, {T}"), tokens::create_a_treasure, "{4}, {T}: Create a Treasure token."),
    ],
    ..oracle::FOUNTAINPORT
};

pub static EXTRA: &[&CardDef] = &[&FISH, &tokens::TREASURE];
