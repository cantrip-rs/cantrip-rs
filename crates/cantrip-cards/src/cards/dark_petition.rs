use cantrip_api::*;
use cantrip_mechanics::{library, select};
use cantrip_oracle as oracle;

/// "Search your library for a card, put that card into your hand, then shuffle. Spell mastery — If there are two or more instant and/or sorcery cards in your graveyard, add {B}{B}{B}." (CR 608.2, 400.7, 701.24a).
fn search_library_card_put_card(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let shown_0 = library::searched_cards(g, item.controller);
    let can_0: Vec<ObjectId> = shown_0.iter().copied().collect();
    let found_0 = library::search(
        g,
        item.controller,
        item.controller,
        item.object,
        SearchDestination::Hand,
        select::Selection::listed(select::Count::AsManyAsCan(1), &shown_0, can_0),
    )?;
    let objects_1 = found_0.clone();
    g.return_to_hand(&objects_1);
    g.shuffle(item.controller);
    g.end_sentence();
    if {
        let q: &dyn Query = g;
        (Some(item.controller)
            .into_iter()
            .flat_map(|p: PlayerId| q.state().zone(Zone::graveyard(p)).to_vec())
            .collect::<Vec<ObjectId>>()
            .into_iter()
            .filter(|&id| {
                q.chars(id).is(CardType::Instant) || q.chars(id).is(CardType::Sorcery)
            })
            .collect::<Vec<ObjectId>>()
            .len() as i64) >= 2
    } {
        g.add_mana(item.controller, Mana::B, 3);
        g.end_sentence();
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[], search_library_card_put_card),
    ..oracle::DARK_PETITION
};
