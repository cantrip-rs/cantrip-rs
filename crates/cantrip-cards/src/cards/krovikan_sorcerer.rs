use cantrip_api::*;
use cantrip_mechanics::{choose, select};
use cantrip_oracle as oracle;

/// "{T}, Discard a nonblack card: Draw a card." (CR 608.2, 121.1).
fn draw_card(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.draw(item.controller, 1);
    Ok(())
}

/// "{T}, Discard a nonblack card: Draw a card." (CR 608.2).
fn draw_card_2(q: &dyn Query, asker: &Asker, id: ObjectId) -> bool {
    !(q.chars(id).colors.has(q.words(asker.source).color(Color::Black)))
}

/// "{T}, Discard a black card: Draw two cards, then discard one of them." (CR 608.2, 121.1, 701.9a).
fn draw_two_cards_discard_one(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let before = g.history().all().len();
    g.draw(item.controller, 2);
    let drawn: Vec<ObjectId> = g
        .history()
        .all()[before..]
        .iter()
        .filter_map(|record| match record.event {
            Event::Drew { player: who, card } if who == item.controller
                && g.state().zone(Zone::hand(item.controller)).contains(&card) => {
                Some(card)
            }
            _ => None,
        })
        .collect();
    let cards = choose::select(
        g,
        item.controller,
        item.object,
        Purpose::Discard,
        drawn,
        select::Selection::new(select::Count::AsManyAsCan(1)),
    )?;
    g.discard_together(&cards);
    Ok(())
}

/// "{T}, Discard a black card: Draw two cards, then discard one of them." (CR 608.2).
fn draw_two_cards_discard_one_2(q: &dyn Query, asker: &Asker, id: ObjectId) -> bool {
    q.chars(id).colors.has(q.words(asker.source).color(Color::Black))
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(
            TAP.and(discard(1, draw_card_2)),
            draw_card,
            "{T}, Discard a nonblack card: Draw a card.",
        ),
        ability(
            TAP.and(discard(1, draw_two_cards_discard_one_2)),
            draw_two_cards_discard_one,
            "{T}, Discard a black card: Draw two cards, then discard one of them.",
        ),
    ],
    ..oracle::KROVIKAN_SORCERER
};
