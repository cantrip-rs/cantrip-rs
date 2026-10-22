//! The gift paths no printed card in the engine reaches, played on a table of test-only cards. Every other kind of gift and the
//! permanent trigger are scenarios of printed cards (Dawn's Truce, Crumb and Get It, Blooming Blast, Into the Flood Maw,
//! Scrapshooter, Octomancer). These are:
//!
//! - "gift an extra turn" (CR 702.174g): the only printed card, Perch Protection, needs phasing and "your life total can't
//!   change", which the engine does not have;
//! - a copy of a permanent spell was promised the gift too (CR 707.10, ruling 2024-07-26): no card the engine has copies a
//!   creature spell;
//! - `Event::GaveGift` (CR 702.174c): no printed card reads it.

use std::collections::VecDeque;
use std::sync::OnceLock;

use cantrip_api::Game as Rules;
use cantrip_engine::*;
use cantrip_mechanics::{gift, tokens};

fn nothing(_: &mut dyn Rules, _: &StackItem) -> Result<(), Suspend> {
    Ok(())
}

/// A zero-cost instant that does nothing, with the gift `gift`.
const fn probe(name: &'static str, gift: GiftDef) -> CardDef {
    CardDef {
        name,
        types: TypeSet::of(&[CardType::Instant]),
        spell: spell(&[], nothing),
        gift: Some(gift),
        ..CardDef::BLANK
    }
}

static EXTRA_TURN: CardDef = probe("Probe Extra Turn", gift::EXTRA_TURN);

/// A land to fill hands and libraries.
static FILLER: CardDef = CardDef { name: "Filler", has_cost: false, types: TypeSet::of(&[CardType::Land]), ..CardDef::BLANK };

/// A zero-cost 1/1 creature with gift: "When this creature enters, if its gift cost was paid, [a tapped Fish]."
static BODY: CardDef = CardDef {
    name: "Probe Body",
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(1),
    toughness: Some(1),
    gift: Some(gift::TAPPED_FISH),
    abilities: &[gift::trigger("When this creature enters, if its gift cost was paid, gift a tapped Fish.")],
    ..CardDef::BLANK
};

fn on_stack(q: &dyn Query, _: &Asker, t: Target) -> bool {
    matches!(t, Target::Object(id) if q.state().stack.iter().any(|i| i.kind == StackKind::Spell && i.object == id))
}

fn copy(g: &mut dyn Rules, item: &StackItem) -> Result<(), Suspend> {
    if let Target::Object(id) = item.target(0) {
        g.copy_spell(id, item.controller, false)?;
    }
    Ok(())
}

/// A zero-cost instant: copy target spell.
static COPIER: CardDef = CardDef { name: "Copier", types: TypeSet::of(&[CardType::Instant]), spell: spell(&[target(on_stack)], copy), ..CardDef::BLANK };

fn table() -> &'static CardTable {
    static TABLE: OnceLock<CardTable> = OnceLock::new();
    TABLE.get_or_init(|| {
        CardTable::new([
            &EXTRA_TURN, &FILLER, &BODY, &COPIER, &tokens::FISH,
        ])
    })
}

/// A two-player game: each seat's hand is the named cards, then lands.
fn deal(first: &[&str], second: &[&str]) -> Game {
    let list = |names: &[&str]| -> Vec<CardId> {
        let mut deck: Vec<CardId> = names.iter().map(|n| table().find(n).unwrap_or_else(|| panic!("no card {n}"))).collect();
        let filler = table().find("Filler").unwrap();
        deck.extend(std::iter::repeat(filler).take(30 - deck.len()));
        deck
    };
    let mut game = Game::new(table(), &Setup { decks: vec![list(first), list(second)], seed: 1, shuffle: false, opening_chooser: None, ..Default::default() });
    game.verify = true;
    game.run();
    game
}

/// Plays on: each seat casts the named cards in order whenever it has priority and can (the first of its list that is
/// castable), says yes to every "you may", takes the first candidate named for a target, and passes otherwise. Stops when
/// `until` holds at a question.
fn play(game: &mut Game, casts: [&[(&str, &str)]; 2], until: impl Fn(&Game) -> bool) {
    let mut plan: [VecDeque<(&str, &str)>; 2] = [casts[0].iter().copied().collect(), casts[1].iter().copied().collect()];
    let mut target_name = "";
    for _ in 0..200 {
        game.run();
        if until(game) {
            return;
        }
        let Some(request) = game.pending().cloned() else { panic!("the game ended") };
        let seat = request.player.0 as usize;
        let answer = match &request.kind {
            RequestKind::Priority { options } => {
                let wanted = plan[seat].front().copied();
                let option = wanted.and_then(|(name, _)| options.iter().find(|o| o.kind == ActionKind::CastSpell && o.source.is_some_and(|s| game.name(s) == name)));
                match option {
                    Some(option) => {
                        let (_, aimed) = plan[seat].pop_front().unwrap();
                        target_name = aimed;
                        let mut action = Action::of(option);
                        if option.targets_required > 0 {
                            let found = option.target_candidates.iter().copied().find(|c| matches!(c, Target::Object(id) if game.name(*id) == aimed));
                            action.targets = vec![found.unwrap_or_else(|| panic!("no candidate named {aimed}"))];
                        }
                        Answer::Act(action)
                    }
                    None => Answer::Pass,
                }
            }
            RequestKind::May { .. } => Answer::Yes,
            RequestKind::ChooseTarget { candidates, .. } => {
                let found = candidates.iter().copied().find(|c| matches!(c, Target::Object(id) if game.name(*id) == target_name));
                Answer::Targets(vec![found.unwrap_or_else(|| panic!("no candidate named {target_name}"))])
            }
            other => housekeeping(other),
        };
        game.answer(request.id, request.player, answer).unwrap();
    }
    let board: Vec<_> = game.state.battlefield().iter().map(|&id| (game.name(id), game.state.obj(id).gift)).collect();
    panic!("the game did not reach the position: battlefield {board:?}, stack {}, gifts {:?}", game.state.stack.len(), game.log.iter().filter(|r| matches!(r.event, Event::GaveGift { .. })).count());
}

/// The answer to a question of the turn's housekeeping that these tests play through.
fn housekeeping(kind: &RequestKind) -> Answer {
    match kind {
        RequestKind::DiscardToHandSize { count, hand } => Answer::Cards(hand.iter().copied().take(*count as usize).collect()),
        RequestKind::DeclareAttackers { .. } => Answer::Attack(Vec::new()),
        RequestKind::DeclareBlockers { .. } => Answer::Block(Vec::new()),
        other => panic!("unexpected question {other:?}"),
    }
}

fn gave(game: &Game, to: u8, gift: Gift) -> usize {
    game.log.iter().filter(|r| matches!(r.event, Event::GaveGift { to: t, gift: g, .. } if t == PlayerId(to) && g == gift)).count()
}

#[test]
fn an_extra_turn_is_taken_by_the_chosen_opponent_after_this_one() {
    let mut game = deal(&["Probe Extra Turn"], &[]);
    play(&mut game, [&[("Probe Extra Turn", "")], &[]], |g| g.state.extra_turns.iter().map(|t| t.player).eq([PlayerId(1)]));
    assert_eq!(gave(&game, 1, Gift::ExtraTurn), 1);
    // Seat 1's turn 2 and then, with nothing else, its extra turn 3; seat 0 resumes on turn 4.
    play(&mut game, [&[], &[]], |g| g.state.turn.number == 3);
    assert_eq!(game.state.turn.active, PlayerId(1));
    assert!(game.state.extra_turns.is_empty());
}

/// A copy of a permanent spell was promised the gift to the same opponent (ruling 2024-07-26): it becomes a token creature,
/// and its trigger gives a gift too.
#[test]
fn a_copied_permanent_spell_keeps_the_promise() {
    let mut game = deal(&["Probe Body", "Copier"], &[]);
    play(&mut game, [&[("Probe Body", ""), ("Copier", "Probe Body")], &[]], |g| gave(g, 1, Gift::TappedFish) == 2 && g.state.stack.is_empty());
    let bodies = game.state.battlefield().iter().filter(|&&id| game.name(id) == "Probe Body").count();
    let fish = game.state.battlefield().iter().filter(|&&id| game.name(id) == "Fish Token").count();
    assert_eq!((bodies, fish), (2, 2));
}
