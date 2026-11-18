use cantrip_api::*;
use cantrip_mechanics::targets;
use cantrip_mechanics::top_of_library;
use cantrip_oracle as oracle;

/// Look at the top three cards of your library, then put them back in any order (no shuffle).
fn look(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let player = item.controller;
    let looked = top_of_library(g, player, 3);
    if looked.is_empty() {
        return Ok(());
    }
    let ask = RequestKind::DistributeCards {
        source: item.object,
        cards: looked.clone(),
        destinations: vec![Destination { kind: DestinationKind::LibraryTop, min: 0, max: u32::MAX, eligible: looked }],
        may_shuffle: false,
    };
    if let Answer::Placements { placements, .. } = g.ask(player, ask)? {
        g.arrange_library(player, &placements[0], &[]);
    }
    Ok(())
}

/// Draw a card, then put this artifact on top of its owner's library. The ability resolves independently if the Top
/// left the battlefield in response (CR 113.7a): then it only draws.
fn draw_and_put_on_top(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.draw(item.controller, 1);
    let top = item.object;
    if targets::permanent(g, top) {
        g.put_into_library(&[top], LibraryPlace::Top);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(cost("{1}"), look, "{1}: Look at the top three cards of your library, then put them back in any order."),
        ability(TAP, draw_and_put_on_top, "{T}: Draw a card, then put Sensei's Divining Top on top of its owner's library."),
    ],
    ..oracle::SENSEIS_DIVINING_TOP
};
