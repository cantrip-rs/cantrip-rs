use cantrip_api::*;
use cantrip_mechanics::{creature_cards_in_graveyard, tokens, triggers};
use cantrip_oracle as oracle;

/// "Create a tapped 2/1 black Villain creature token with menace if there are two or more creature cards in your graveyard.
/// Otherwise, mill two cards." (CR 608.2c, 701.17a): the graveyard is counted as the ability resolves, in its controller's
/// graveyard alone, tokens not being cards; one branch or the other, never both. The troopers are on the battlefield, so they
/// do not count themselves.
fn villain_or_mill(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if creature_cards_in_graveyard(g, item.controller) >= 2 {
        tokens::create_by_rule(g, item.controller, &tokens::VILLAIN_2_1_BLACK_MENACE, 1, true)?;
    } else {
        g.mill(item.controller, 2);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::THIS_ENTERS,
        villain_or_mill,
        "When this creature enters, create a tapped 2/1 black Villain creature token with menace if there are two or more creature cards in your graveyard. Otherwise, mill two cards.",
    )],
    ..oracle::HYDRA_TROOPERS
};

pub static EXTRA: &[&CardDef] = &[&tokens::VILLAIN_2_1_BLACK_MENACE];
