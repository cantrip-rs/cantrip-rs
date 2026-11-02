use cantrip_api::*;
use cantrip_mechanics::{choose, select, choose_from_hand};
use cantrip_oracle as oracle;

/// "Draw three cards, untap up to two lands, then discard a card." (CR 608.2, 121.1, 701.9a).
fn draw_three_cards_untap_up(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.draw(item.controller, 3);
    g.end_sentence();
    let selected_1_candidates = {
        let q: &dyn Query = g;
        q.state()
            .battlefield()
            .iter()
            .copied()
            .filter(|&id| q.chars(id).is(CardType::Land))
            .collect::<Vec<ObjectId>>()
    };
    let selected_1 = choose::select(
        g,
        item.controller,
        item.object,
        Purpose::Untap,
        selected_1_candidates,
        select::Selection::of(select::Count::UpTo(2), &[]),
    )?;
    let objects_2 = {
        let q: &dyn Query = g;
        selected_1
            .clone()
            .into_iter()
            .filter(|&o| q.state().obj(o).became.is_none())
            .collect::<Vec<ObjectId>>()
    };
    let turned_2: Vec<ObjectId> = objects_2
        .iter()
        .copied()
        .filter(|&o| g.state().obj(o).tapped != false)
        .collect();
    instruction(
        g,
        |g| {
            for &object in &turned_2 {
                g.untap(object);
            }
        },
    );
    g.end_sentence();
    let mut cards_3 = Vec::new();
    cards_3.extend(choose_from_hand(g, item.controller, item.object, 1)?);
    g.discard_together(&cards_3);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[], draw_three_cards_untap_up),
    ..oracle::PORE_OVER_THE_PAGES
};
