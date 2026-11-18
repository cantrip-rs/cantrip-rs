use cantrip_api::*;
use cantrip_mechanics::top_of_library;
use cantrip_oracle as oracle;

/// Look at the top three cards of your library, then put them back in any order; you may shuffle. Draw a card.
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let player = item.controller;
    let looked = top_of_library(g, player, 3);
    if !looked.is_empty() {
        let ask = RequestKind::DistributeCards {
            source: item.object,
            cards: looked.clone(),
            destinations: vec![Destination { kind: DestinationKind::LibraryTop, min: 0, max: u32::MAX, eligible: looked }],
            may_shuffle: true,
        };
        if let Answer::Placements { placements, shuffle } = g.ask(player, ask)? {
            if shuffle {
                g.shuffle(player);
            } else {
                g.arrange_library(player, &placements[0], &[]);
            }
        }
    }
    g.draw(player, 1);
    Ok(())
}

pub static CARD: CardDef = CardDef { spell: spell(&[], effect), ..oracle::PONDER };
