//! "Each player loses 2 life" (Risky Shortcut, Impossible Choice, The City on the Edge of Forever; CR 101.4, 119.3) is one
//! simultaneous event for every player (CR 603.2c), not one event per player: a "whenever one or more players lose life"
//! ability (Emet-Selch of the Third Seat's kind) triggers once in a three-player game. No implemented card has that trigger
//! yet, so a test card does here.

use std::sync::OnceLock;

use cantrip_api::Game as Rules;
use cantrip_engine::*;

/// "Whenever one or more players lose life": any life loss, whoever suffers it.
fn players_lose_life(_: &dyn Query, _: ObjectId, event: &Event) -> Option<Fired> {
    match event {
        Event::LifeChange { delta, .. } if *delta < 0 => Fired::PLAIN,
        _ => None,
    }
}

fn gain_one(g: &mut dyn Rules, item: &StackItem) -> Result<(), Suspend> {
    g.gain_life(item.controller, 1);
    Ok(())
}

/// A zero-cost enchantment: "Whenever one or more players lose life, you gain 1 life."
static WITNESS: CardDef = CardDef {
    name: "Probe Witness",
    types: TypeSet::of(&[CardType::Enchantment]),
    abilities: &[triggered(when(EventMask::LIFE_CHANGE, players_lose_life), gain_one, "Whenever one or more players lose life, you gain 1 life.")
        .once_per_batch()],
    ..CardDef::BLANK
};

/// "Draw two cards. Each player loses 2 life."
static DRAIN: CardDef = CardDef {
    name: "Probe Drain",
    types: TypeSet::of(&[CardType::Sorcery]),
    spell: spell(&[], cantrip_mechanics::effects::draw_two_each_player_loses_two),
    ..CardDef::BLANK
};
static FILLER: CardDef = CardDef { name: "Filler", has_cost: false, types: TypeSet::of(&[CardType::Land]), ..CardDef::BLANK };

fn table() -> &'static CardTable {
    static TABLE: OnceLock<CardTable> = OnceLock::new();
    TABLE.get_or_init(|| CardTable::new([&WITNESS, &DRAIN, &FILLER]))
}

#[test]
fn every_player_losing_life_is_one_event_that_triggers_once() {
    let find = |n: &str| table().find(n).unwrap();
    let mut first = vec![find("Probe Witness"), find("Probe Drain")];
    first.extend(std::iter::repeat_n(find("Filler"), 28));
    let other = vec![find("Filler"); 30];
    let mut game = Game::new(table(), &Setup { decks: vec![first, other.clone(), other], seed: 1, shuffle: false, opening_chooser: None, ..Default::default() });
    game.verify = true;
    let mut casts = ["Probe Witness", "Probe Drain"].into_iter().peekable();
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
    // Each lost 2; the watcher's controller gained 1 once, for the one event.
    assert_eq!(game.state.player(PlayerId(0)).life, 20 - 2 + 1);
    assert_eq!(game.state.player(PlayerId(1)).life, 20 - 2);
    assert_eq!(game.state.player(PlayerId(2)).life, 20 - 2);
}
