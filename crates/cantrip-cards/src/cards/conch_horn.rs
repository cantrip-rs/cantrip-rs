use cantrip_api::*;
use cantrip_mechanics::{choose, select};
use cantrip_oracle as oracle;

/// "{1}, {T}, Sacrifice this artifact: Draw two cards, then put a card from your hand on top of your library." (CR 608.2, 121.1).
fn draw_two_cards_put_card(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.draw(item.controller, 2);
    g.end_sentence();
    let selected_1_candidates = {
        let q: &dyn Query = g;
        Some(item.controller)
            .into_iter()
            .flat_map(|p: PlayerId| q.state().zone(Zone::hand(p)).to_vec())
            .collect::<Vec<ObjectId>>()
            .into_iter()
            .collect::<Vec<ObjectId>>()
    };
    let selected_1 = choose::select(
        g,
        item.controller,
        item.object,
        Purpose::PutOnTop,
        selected_1_candidates,
        select::Selection::of(select::Count::AsManyAsCan(1), &[]),
    )?;
    let objects_2 = selected_1.clone().into_iter().rev().collect::<Vec<ObjectId>>();
    g.put_into_library(&objects_2, LibraryPlace::Top);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(
            cost("{1}").and(SACRIFICE_THIS).and(TAP),
            draw_two_cards_put_card,
            "{1}, {T}, Sacrifice this artifact: Draw two cards, then put a card from your hand on top of your library.",
        ),
    ],
    ..oracle::CONCH_HORN
};
