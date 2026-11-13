//! Ob Nixilis, the Adversary's +1, "Each opponent loses 2 life unless they discard a card" (CR 101.4, 119.3): each opponent
//! in turn order chooses, then the discards are done together and the life losses together, one event (CR 603.2c), not one
//! event per opponent. A "whenever one or more opponents lose life" ability (Emet-Selch of the Third Seat's kind) triggers
//! once in a three-player game where both opponents keep their cards. No implemented card has that trigger yet, so a test
//! card does here, and a test sorcery resolves Ob Nixilis's own +1 effect.

use std::sync::OnceLock;

use cantrip_api::Game as Rules;
use cantrip_engine::*;

/// "Whenever one or more opponents lose life": a life loss of anyone but the ability's controller.
fn opponents_lose_life(_: &dyn Query, _: ObjectId, event: &Event) -> Option<Fired> {
    match event {
        Event::LifeChange { player, delta } if *delta < 0 && *player != PlayerId(0) => Fired::PLAIN,
        _ => None,
    }
}

fn gain_one(g: &mut dyn Rules, item: &StackItem) -> Result<(), Suspend> {
    g.gain_life(item.controller, 1);
    Ok(())
}

/// A zero-cost enchantment: "Whenever one or more opponents lose life, you gain 1 life."
static WITNESS: CardDef = CardDef {
    name: "Probe Witness",
    types: TypeSet::of(&[CardType::Enchantment]),
    abilities: &[triggered(when(EventMask::LIFE_CHANGE, opponents_lose_life), gain_one, "Whenever one or more opponents lose life, you gain 1 life.")
        .once_per_batch()],
    ..CardDef::BLANK
};

/// Ob Nixilis, the Adversary's +1 effect, as the card's file defines it.
fn ob_nixilis_plus_one(g: &mut dyn Rules, item: &StackItem) -> Result<(), Suspend> {
    static EFFECT: OnceLock<Effect> = OnceLock::new();
    let effect = EFFECT.get_or_init(|| {
        let ob = cantrip_cards::all().into_iter().find(|c| c.name == "Ob Nixilis, the Adversary").expect("Ob Nixilis is implemented");
        ob.abilities
            .iter()
            .find_map(|a| match a {
                AbilityDef::Activated(a) if a.text.starts_with("+1:") => Some(a.effect),
                _ => None,
            })
            .expect("Ob Nixilis has a +1")
    });
    effect(g, item)
}

/// A zero-cost sorcery with Ob Nixilis's +1.
static PLUS_ONE: CardDef = CardDef {
    name: "Probe Plus One",
    types: TypeSet::of(&[CardType::Sorcery]),
    spell: spell(&[], ob_nixilis_plus_one),
    ..CardDef::BLANK
};
static FILLER: CardDef = CardDef { name: "Filler", has_cost: false, types: TypeSet::of(&[CardType::Land]), ..CardDef::BLANK };

fn table() -> &'static CardTable {
    static TABLE: OnceLock<CardTable> = OnceLock::new();
    TABLE.get_or_init(|| CardTable::new([&WITNESS, &PLUS_ONE, &FILLER]))
}

#[test]
fn opponents_who_keep_their_cards_lose_life_in_one_event_that_triggers_once() {
    let find = |n: &str| table().find(n).unwrap();
    let mut first = vec![find("Probe Witness"), find("Probe Plus One")];
    first.extend(std::iter::repeat_n(find("Filler"), 28));
    let other = vec![find("Filler"); 30];
    let mut game = Game::new(table(), &Setup { decks: vec![first, other.clone(), other], seed: 1, shuffle: false, opening_chooser: None, ..Default::default() });
    game.verify = true;
    let mut casts = ["Probe Witness", "Probe Plus One"].into_iter().peekable();
    let mut asked = Vec::new();
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
            RequestKind::May { .. } => {
                asked.push(request.player);
                Answer::No
            }
            RequestKind::OrderTriggers { sources, .. } => Answer::Order((0..count_u32(sources.len())).collect()),
            other => panic!("unexpected question {other:?}"),
        };
        game.answer(request.id, request.player, answer).unwrap();
    }
    // Each opponent was asked, in turn order, and kept their cards.
    assert_eq!(asked, [PlayerId(1), PlayerId(2)]);
    assert_eq!(game.state.player(PlayerId(1)).life, 20 - 2);
    assert_eq!(game.state.player(PlayerId(2)).life, 20 - 2);
    // The watcher's controller gained 1 once, for the one event.
    assert_eq!(game.state.player(PlayerId(0)).life, 20 + 1);
}
