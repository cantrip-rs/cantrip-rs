use cantrip_api::*;
use cantrip_mechanics::{library, mana, select, targets};
use cantrip_oracle as oracle;

/// "{1}, {T}, Sacrifice this artifact: Target opponent puts the cards from their hand on top of their library. Search that player's library for that many cards. That player puts those cards into their hand, then shuffles." (CR 608.2, 400.7, 701.24a).
fn target_opponent_puts_cards_their(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let Some(player) = targets::player(g, item, 0) else { return Ok(()) };
    let hand = g.state().zone(Zone::hand(player)).to_vec();
    let order = library::ask_library_order(g, player, item.object, &hand, DestinationKind::LibraryTop)?;
    let moved = g.put_into_library(&order, LibraryPlace::Top);
    let count = moved.iter().filter(|&&id| g.state().obj(id).zone == Zone::library(player)).count() as u32;
    g.end_sentence();
    let found = library::search(g, item.controller, player, item.object, SearchDestination::Hand, select::cards(select::Count::AsManyAsCan(count)))?;
    g.return_to_hand(&found);
    g.shuffle(player);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(
                cost("{1}").and(SACRIFICE_THIS).and(TAP),
                target_opponent_puts_cards_their,
                "{1}, {T}, Sacrifice this artifact: Target opponent puts the cards from their hand on top of their library. Search that player's library for that many cards. That player puts those cards into their hand, then shuffles.",
            )
            .targets(&[targets::OPPONENT]),
    ],
    enters: Some(mana::enters_tapped),
    ..oracle::JESTERS_MASK
};
