use cantrip_api::*;
use cantrip_mechanics::{choose, effects, select};
use cantrip_oracle as oracle;

/// I, II, III: "Draw two cards, then exile a card from your hand." (CR 608.2c: the draw is done before the choice is
/// made.) The exiled card is exiled with this Saga (CR 607.2a), for chapter IV to find. Nothing is exiled from an empty hand.
fn draw_two_then_exile(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let player = item.controller;
    g.draw(player, 2);
    g.end_sentence();
    let hand = g.state().zone(Zone::hand(player)).to_vec();
    let cards = choose::select(g, player, item.object, Purpose::Exile, hand, select::cards(select::exactly(1)))?;
    g.exile_with(&cards, g.linked(item), false);
    Ok(())
}

/// IV: "If all cards exiled with this Saga share a card type, you gain 10 life." (CR 607.2a.) The cards share a type when
/// some card type is on every one of them; with no cards exiled (Binding Negotiation took them) that holds vacuously.
fn chapter_four(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let exiled = g.exiled_with(g.linked(item));
    if select::objects_share_a_card_type(&*g, &exiled) {
        effects::gain_life(g, item, 10)?;
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        chapter(1, draw_two_then_exile, "I \u{2014} Draw two cards, then exile a card from your hand."),
        chapter(2, draw_two_then_exile, "II \u{2014} Draw two cards, then exile a card from your hand."),
        chapter(3, draw_two_then_exile, "III \u{2014} Draw two cards, then exile a card from your hand."),
        chapter(4, chapter_four, "IV \u{2014} If all cards exiled with this Saga share a card type, you gain 10 life."),
    ],
    ..oracle::ALL_GOOD_THINGS
};
