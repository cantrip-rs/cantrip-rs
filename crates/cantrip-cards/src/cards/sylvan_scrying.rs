use cantrip_api::*;
use cantrip_mechanics::{library, select};
use cantrip_oracle as oracle;

/// "Search your library for a land card, reveal it, put it into your hand, then shuffle." (CR 608.2, 400.7, 701.24a).
fn search_library_land_card_reveal(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let shown_0 = library::searched_cards(g, item.controller);
    let can_0: Vec<ObjectId> = {
        let q: &dyn Query = g;
        shown_0.iter().copied().filter(|&id| q.chars(id).is(CardType::Land)).collect()
    };
    let found_0 = library::search(
        g,
        item.controller,
        item.controller,
        item.object,
        SearchDestination::Hand,
        select::Selection::listed(select::Count::UpTo(1), &shown_0, can_0),
    )?;
    let objects_1 = found_0.clone();
    let mut owners_1: Vec<PlayerId> = objects_1
        .iter()
        .map(|o| g.state().obj(*o).owner)
        .collect();
    owners_1.dedup();
    for owner in owners_1 {
        let cards: Vec<ObjectId> = objects_1
            .iter()
            .copied()
            .filter(|o| g.state().obj(*o).owner == owner)
            .collect();
        g.reveal(owner, &cards);
    }
    let objects_2 = found_0.clone();
    g.return_to_hand_revealed(&objects_2);
    g.shuffle(item.controller);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[], search_library_land_card_reveal),
    ..oracle::SYLVAN_SCRYING
};
