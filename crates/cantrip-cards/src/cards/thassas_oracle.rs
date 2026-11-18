use cantrip_api::*;
use cantrip_mechanics::{devotion::devotion, top_of_library, triggers::THIS_ENTERS};
use cantrip_oracle as oracle;

/// "Look at the top X cards of your library, where X is your devotion to blue. Put up to one of them on top of your
/// library and the rest on the bottom of your library in a random order. If X is greater than or equal to the number of
/// cards in your library, you win the game."
fn look_and_win(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let you = item.controller;
    let color = g.words(item.object).color(Color::Blue);
    let devotion = devotion(g, you, color);
    let looked = top_of_library(g, you, devotion as usize);
    if !looked.is_empty() {
        let ask = RequestKind::DistributeCards {
            source: item.object,
            cards: looked.clone(),
            destinations: vec![
                Destination { kind: DestinationKind::LibraryTop, min: 0, max: 1, eligible: looked.clone() },
                Destination { kind: DestinationKind::LibraryBottom, min: 0, max: u32::MAX, eligible: looked },
            ],
            may_shuffle: false,
        };
        if let Answer::Placements { placements, .. } = g.ask(you, ask)? {
            g.arrange_library(you, &placements[0], &[]);
            g.bottom_in_random_order(you, &placements[1]);
        }
    }
    if devotion as usize >= g.state().zone(Zone::library(you)).len() {
        g.win_game(you);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(THIS_ENTERS, look_and_win, "When this creature enters, look at the top X cards of your library, where X is your devotion to blue. Put up to one of them on top of your library and the rest on the bottom of your library in a random order. If X is greater than or equal to the number of cards in your library, you win the game.")],
    ..oracle::THASSAS_ORACLE
};
