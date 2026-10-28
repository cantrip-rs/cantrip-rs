use cantrip_api::*;
use cantrip_mechanics::{choose, delayed, library, select, targets, triggers};
use cantrip_oracle as oracle;

/// "Basic Plains card" (CR 205.3i, 305.6): a card with the basic supertype and the Plains subtype, Plains read through the
/// words of this Saga's text (CR 612), each card's own.
fn basic_plains(q: &dyn Query, choice: &Choice, _: u32) -> Verdict {
    let plains = q.words(choice.source).subtype(Subtype::Plains);
    Verdict::each(choice, |card| {
        let chars = q.chars(card);
        chars.supertypes.basic && chars.has_subtype(plains)
    })
}

const UP_TO_TWO_BASIC_PLAINS: Selection = Selection::of(select::up_to(2), &[select::part(&(basic_plains as SelectFn), 0, "")]).called("basic Plains");

/// I: "Search your library for up to two basic Plains cards, exile them, then shuffle. You gain 2 life." (CR 701.23, 701.13, 701.24.)
/// The cards found are exiled with this Saga (CR 607.2a); the search may find fewer, or none (CR 701.23b), and the shuffle and
/// the life happen either way.
fn chapter_one(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let player = item.controller;
    let found = library::search(g, player, player, item.object, SearchDestination::Exile, UP_TO_TWO_BASIC_PLAINS)?;
    g.exile_with(&found, g.linked(item), false);
    g.shuffle(player);
    g.end_sentence();
    g.gain_life(player, 2);
    Ok(())
}

/// II, III: "Put a card exiled with this Saga into its owner's hand." (CR 607.2a, 400.3.) The controller chooses among the cards
/// still exiled with it; nothing happens when there are none.
fn put_an_exiled_card_into_hand(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let exiled = g.exiled_with(g.linked(item));
    let chosen = choose::select(g, item.controller, item.object, Purpose::PutIntoHand, exiled, select::cards(select::exactly(1)))?;
    g.return_to_hand(&chosen);
    Ok(())
}

/// "Target creature you control gets +1/+1 until end of turn for each Plains you control": the Plains counted as this resolves,
/// Plains read through the words of this Saga's text (CR 612). Nothing happens if the creature is no longer a legal target (CR
/// 608.2b).
fn grow_for_each_plains(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    let plains = g.words(item.object).subtype(Subtype::Plains);
    let count = g.permanents_of(item.controller).into_iter().filter(|&id| g.chars(id).has_subtype(plains)).count() as i16;
    cantrip_mechanics::pump(g, item.object, creature, count, count);
    Ok(())
}

/// IV: "Whenever you attack this turn, target creature you control gets +1/+1 until end of turn for each Plains you control."
/// A delayed triggered ability (CR 603.7b) for the rest of the turn, controlled by the player whose chapter this is.
fn chapter_four(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    delayed::this_turn(g, item, item.object);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        chapter(1, chapter_one, "I \u{2014} Search your library for up to two basic Plains cards, exile them, then shuffle. You gain 2 life."),
        chapter(2, put_an_exiled_card_into_hand, "II \u{2014} Put a card exiled with this Saga into its owner's hand."),
        chapter(3, put_an_exiled_card_into_hand, "III \u{2014} Put a card exiled with this Saga into its owner's hand."),
        chapter(4, chapter_four, "IV \u{2014} Whenever you attack this turn, target creature you control gets +1/+1 until end of turn for each Plains you control."),
    ],
    delayed: &[triggered(triggers::YOU_ATTACK, grow_for_each_plains, "Whenever you attack this turn, target creature you control gets +1/+1 until end of turn for each Plains you control.")
        .targets(&[targets::CREATURE_YOU_CONTROL])],
    ..oracle::ROADS_GO_EVER_EVER_ON
};
