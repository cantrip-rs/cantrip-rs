use cantrip_api::*;
use cantrip_mechanics::{choose, choose_from_hand, select};
use cantrip_oracle as oracle;

/// "Discard X cards, then return a card from your graveyard to your hand for each card discarded this way. Exile Recall." (CR 608.2, 701.13a, 701.9a).
fn discard_x_cards_return_card(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let cards = choose_from_hand(g, item.controller, item.object, u32::from(item.x))?;
    let discarded = g.discard_together(&cards);
    g.end_sentence();
    if !discarded.is_empty() {
        let candidates: Vec<ObjectId> = g.state().zone(Zone::graveyard(item.controller)).iter().copied()
            .filter(|&id| !g.obj(id).token && !g.obj(id).card_copy)
            .collect();
        let selected = choose::select(
            g, item.controller, item.object, Purpose::PutIntoHand, candidates,
            select::cards(select::Count::AsManyAsCan(discarded.len() as u32)),
        )?;
        g.return_to_hand(&selected);
    }
    g.end_sentence();
    g.exile_all(&[item.object]);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[], discard_x_cards_return_card),
    ..oracle::RECALL
};
