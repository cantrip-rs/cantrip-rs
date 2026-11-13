//! A card that has left its hand since it was named is a new object (CR 400.7): discarding the old one, by `discard_together`
//! or by `Game::together`, discards nothing, and no `Discarded` event names the card where it went - neither the move nor
//! "whenever a player discards a card" (CR 701.9a) sees a second discard.

use cantrip_api::Game as Rules;
use cantrip_engine::*;

/// Every card discarded.
fn a_card_is_discarded(_: &dyn Query, _: ObjectId, event: &Event) -> Option<Fired> {
    match event {
        Event::Discarded { .. } => Fired::PLAIN,
        _ => None,
    }
}

fn gain_one(g: &mut dyn Rules, item: &StackItem) -> Result<(), Suspend> {
    g.gain_life(item.controller, 1);
    Ok(())
}

/// A zero-cost enchantment: "Whenever a player discards a card, you gain 1 life."
static WITNESS: CardDef = CardDef {
    name: "Probe Discard Witness",
    types: TypeSet::of(&[CardType::Enchantment]),
    abilities: &[triggered(when(EventMask::DISCARDED, a_card_is_discarded), gain_one, "Whenever a player discards a card, you gain 1 life.")],
    ..CardDef::BLANK
};

/// Discards the first card of its controller's hand, then that card's old id again, alone and with `together`.
fn discard_then_discard_it_again(g: &mut dyn Rules, item: &StackItem) -> Result<(), Suspend> {
    let card = g.state().zone(Zone::hand(item.controller))[0];
    g.discard_together(&[card]);
    g.end_sentence();
    g.discard_together(&[card]);
    g.end_sentence();
    g.together(Together { discards: &[card], ..Together::default() });
    Ok(())
}

static DISCARD_TWICE: CardDef = CardDef {
    name: "Probe Discard Twice",
    types: TypeSet::of(&[CardType::Sorcery]),
    spell: spell(&[], discard_then_discard_it_again),
    ..CardDef::BLANK
};
static FILLER: CardDef = CardDef { name: "Filler", has_cost: false, types: TypeSet::of(&[CardType::Land]), ..CardDef::BLANK };

#[test]
fn a_card_discarded_once_is_not_discarded_again_by_its_old_id() {
    let table: &'static CardTable = Box::leak(Box::new(CardTable::new([&WITNESS, &DISCARD_TWICE, &FILLER])));
    let find = |n: &str| table.find(n).unwrap();
    let mut first = vec![find("Probe Discard Witness"), find("Probe Discard Twice")];
    first.extend(std::iter::repeat_n(find("Filler"), 28));
    let other = vec![find("Filler"); 30];
    let mut game = Game::new(table, &Setup { decks: vec![first, other], seed: 1, shuffle: false, opening_chooser: None, ..Default::default() });
    game.verify = true;
    let mut casts = ["Probe Discard Witness", "Probe Discard Twice"].into_iter().peekable();
    for _ in 0..200 {
        game.run();
        if casts.peek().is_none() && game.state.stack.is_empty() && game.state.triggers.is_empty() && !game.pending().is_some_and(|r| matches!(r.kind, RequestKind::OrderTriggers { .. })) {
            break;
        }
        let request = game.pending().cloned().expect("the game goes on");
        let answer = match &request.kind {
            RequestKind::Priority { options } if request.player == PlayerId(0) && game.state.stack.is_empty() => {
                match casts.peek().and_then(|&n| options.iter().find(|o| o.kind == ActionKind::CastSpell && o.source.is_some_and(|s| game.name(s) == n))) {
                    Some(option) => {
                        casts.next();
                        Answer::Act(Action::of(option))
                    }
                    None => Answer::Pass,
                }
            }
            RequestKind::Priority { .. } => Answer::Pass,
            RequestKind::OrderTriggers { sources, .. } => Answer::Order((0..count_u32(sources.len())).collect()),
            other => panic!("unexpected question {other:?}"),
        };
        game.answer(request.id, request.player, answer).unwrap();
    }
    // One card discarded, once: the witness gained 1 life.
    assert_eq!(game.state.player(PlayerId(0)).life, 21);
    assert_eq!(game.state.zone(Zone::graveyard(PlayerId(0))).len(), 2, "the discarded card and the sorcery");
}
