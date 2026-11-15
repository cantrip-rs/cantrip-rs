use cantrip_api::*;
use cantrip_mechanics::{choose, select, triggers};
use cantrip_oracle as oracle;

/// "A legendary and/or enchantment card" (CR 205.4, 205.2a): a card with the legendary supertype, the enchantment type, or both.
fn legendary_and_or_enchantment(q: &dyn Query, choice: &Choice, _: u32) -> Verdict {
    Verdict::each(choice, |card| {
        let chars = q.chars(card);
        chars.supertypes.legendary || chars.is(CardType::Enchantment)
    })
}

/// "You mill that many cards. You may put any number of legendary and/or enchantment cards from among them into your hand.
/// If you put no cards into your hand this way, put a +1/+1 counter on Nashi." The cards milled are the ones shown (CR 701.17c);
/// none may be taken, and then the counter goes on (CR 608.2d). A library with fewer cards mills what it has.
fn mill_take_or_grow(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let you = item.controller;
    let available = g.state().zone(Zone::library(you)).len();
    let count = item.matched_damage_amount().min(available.min(u32::MAX as usize) as u32);
    let milled = g.mill(you, count);
    g.end_sentence();
    let rule = select::cards(select::ANY_NUMBER)
        .with(select::part(&(legendary_and_or_enchantment as SelectFn), 0, ""))
        .called("legendary and/or enchantment");
    let chosen = choose::select(g, you, item.object, Purpose::PutIntoHand, milled, rule)?;
    if g.return_to_hand(&chosen).is_empty() {
        g.end_sentence();
        g.add_counters(item.object, CounterKind::PlusOne, 1);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(triggers::DEALS_COMBAT_DAMAGE_TO_PLAYER_EVENT, mill_take_or_grow, "Whenever this creature deals combat damage to a player, you mill that many cards. You may put any number of legendary and/or enchantment cards from among them into your hand. If you put no cards into your hand this way, put a +1/+1 counter on this creature.").damage_amount()],
    ..oracle::NASHI_SEARCHER_IN_THE_DARK
};
