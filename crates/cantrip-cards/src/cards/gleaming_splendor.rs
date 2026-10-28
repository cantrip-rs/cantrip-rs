use cantrip_api::*;
use cantrip_mechanics::{targets, tokens, triggers};
use cantrip_oracle as oracle;

/// "You create a Treasure token" (CR 111.10a).
fn create_treasure(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create_treasure(g, item.controller)
}

/// "Two target players each draw a card" (CR 121.1, 115.3): the targets still legal as it resolves draw, in turn order from the
/// active player (CR 101.4).
fn two_players_draw(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let players = targets::legal_players(g, item);
    let ordered: Vec<PlayerId> = g.state().apnap().filter(|p| players.contains(p)).collect();
    for player in ordered {
        g.draw(player, 1);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(triggers::AN_OPPONENT_DRAWS_THEIR_SECOND_CARD, create_treasure, "Whenever an opponent draws their second card each turn, you create a Treasure token."),
        ability(cost("{2}{W}"), two_players_draw, "{2}{W}: Two target players each draw a card.").targets(&[targets::PLAYER.between(2, 2)]),
    ],
    ..oracle::GLEAMING_SPLENDOR
};

pub static EXTRA: &[&CardDef] = &[&tokens::TREASURE];
