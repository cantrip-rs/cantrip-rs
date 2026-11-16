//! Modes counted in pawprints (CR 700.2i: "the spell's controller can choose any number of modes such that the total number of
//! pawprint symbols on the chosen modes is not greater than the specified number"; CR 107.18; CR 700.2d: the same mode chosen
//! more than once is that many instances, in printed order, each with targets of its own). Over a test card of the Seasons'
//! shape: {P} you gain 1 life, {P}{P} target player loses 2 life, {P}{P}{P} you gain life equal to your life total.

use crate::{Game, Setup};
use cantrip_api::*;
use cantrip_mechanics::{copy, targets};
use cantrip_state::{Frame, FrameKind};

const YOU: PlayerId = PlayerId(0);
const THEM: PlayerId = PlayerId(1);

fn gain_one(g: &mut dyn cantrip_api::Game, item: &StackItem) -> Result<(), Suspend> {
    g.gain_life(item.controller, 1);
    Ok(())
}

fn target_loses_two(g: &mut dyn cantrip_api::Game, item: &StackItem) -> Result<(), Suspend> {
    if let Some(player) = targets::player(g, item, 0) {
        g.lose_life(player, 2);
    }
    Ok(())
}

/// Order matters to it: after "gain 1" it doubles one more.
fn double_life(g: &mut dyn cantrip_api::Game, item: &StackItem) -> Result<(), Suspend> {
    let life = g.state().player(item.controller).life;
    g.gain_life(item.controller, life);
    Ok(())
}

static FILLER: CardDef = CardDef { name: "Pawprint Test Filler", has_cost: false, types: TypeSet::of(&[CardType::Land]), ..CardDef::BLANK };

/// "Choose up to five {P} worth of modes. You may choose the same mode more than once."
static SEASON_MODES: ModalDef = modes(&[
    mode(gain_one, "{P} — You gain 1 life.").pawprints(1),
    mode(target_loses_two, "{P}{P} — Target player loses 2 life.").targets(&[targets::PLAYER]).pawprints(2),
    mode(double_life, "{P}{P}{P} — You gain life equal to your life total.").pawprints(3),
])
.pawprints(5);

static SEASON: CardDef = CardDef {
    name: "Pawprint Test Season",
    types: TypeSet::of(&[CardType::Sorcery]),
    spell: Some(SpellDef::MODAL),
    modal: Some(&SEASON_MODES),
    ..CardDef::BLANK
};

/// "Copy target instant or sorcery spell. You may choose new targets for the copy."
static TWIN: CardDef = CardDef {
    name: "Pawprint Test Twin",
    types: TypeSet::of(&[CardType::Instant]),
    spell: spell(&[targets::INSTANT_OR_SORCERY_SPELL], copy::copy_target_spell),
    ..CardDef::BLANK
};

/// "{P}{P} - Target creature ..." its only mode: with no creature to target, no mode could be chosen.
static LONE_MODES: ModalDef =
    modes(&[mode(gain_one, "{P}{P} — Choose target creature. You gain 1 life.").targets(&[targets::CREATURE]).pawprints(2)]).pawprints(5);

static LONE: CardDef = CardDef {
    name: "Pawprint Test Lone Season",
    types: TypeSet::of(&[CardType::Sorcery]),
    spell: Some(SpellDef::MODAL),
    modal: Some(&LONE_MODES),
    ..CardDef::BLANK
};

/// Your main phase with priority, the Season and the copier in your hand.
fn game() -> (Game, ObjectId, ObjectId) {
    let table: &'static CardTable = Box::leak(Box::new(CardTable::new([&FILLER, &SEASON, &TWIN])));
    let (g, hand) = game_with(table, &[&SEASON, &TWIN]);
    (g, hand[0], hand[1])
}

/// Your main phase with priority, `hand` in your hand, in that order.
fn game_with(table: &'static CardTable, hand: &[&'static CardDef]) -> (Game, Vec<ObjectId>) {
    let deck = || vec![table.id_of(&FILLER); 20];
    let mut g = Game::new(table, &Setup { decks: vec![deck(), deck()], ..Default::default() });
    while !g.state.agenda.is_empty() {
        g.state.frame_remove(0);
    }
    g.state.update_turn(|t| {
        t.active = YOU;
        t.priority = YOU;
        t.set_step(Step::PrecombatMain);
    });
    g.state.frame_push(Frame::new(FrameKind::Priority));
    let mut ids = Vec::new();
    for &def in hand {
        let id = g.state.push_obj(Obj::new(table.id_of(def), YOU, Zone::hand(YOU)));
        g.state.zone_push(Zone::hand(YOU), id);
        ids.push(id);
    }
    (g, ids)
}

fn next(g: &mut Game) -> Request {
    g.run();
    g.pending().cloned().expect("a question")
}

fn cast(g: &mut Game, card: ObjectId) {
    let r = next(g);
    let RequestKind::Priority { options } = &r.kind else { panic!("expected priority, got {:?}", r.kind) };
    let option = options.iter().find(|o| o.kind == ActionKind::CastSpell && o.source == Some(card)).expect("castable").clone();
    // A spell with one target is cast at the first candidate: the copier's is the Season.
    let targets = option.target_candidates.iter().take(option.targets_required as usize).copied().collect();
    g.answer(r.id, r.player, Answer::Act(Action { targets, ..Action::of(&option) })).expect("a legal answer");
}

/// Casts the Season with `chosen` modes, each player target answered in turn from `aim`, and pays.
fn cast_season(g: &mut Game, season: ObjectId, chosen: &[u8], aim: &[PlayerId]) {
    cast(g, season);
    let r = next(g);
    assert!(matches!(r.kind, RequestKind::ChooseModes { .. }), "the modes are asked, got {:?}", r.kind);
    g.answer(r.id, r.player, Answer::Modes(chosen.to_vec())).expect("a legal choice of modes");
    let mut aim = aim.iter();
    loop {
        let r = next(g);
        match &r.kind {
            RequestKind::ChooseTarget { .. } => {
                let who = *aim.next().expect("a target to aim");
                g.answer(r.id, r.player, Answer::Targets(vec![Target::Player(who)])).expect("a legal target");
            }
            RequestKind::PayCosts { options, .. } => {
                let pay = options.iter().find(|o| matches!(o.kind, ActionKind::ConfirmCasting | ActionKind::AssignMana)).expect("it pays").clone();
                g.answer(r.id, r.player, Answer::Act(Action::of(&pay))).expect("a legal answer");
            }
            _ => break,
        }
    }
    assert!(aim.next().is_none(), "every target was asked");
}

/// Passes until the stack is empty.
fn resolve(g: &mut Game) {
    for _ in 0..12 {
        let r = next(g);
        if g.state.stack.is_empty() {
            return;
        }
        g.answer(r.id, r.player, Answer::Pass).unwrap_or_else(|e| panic!("{e:?} for {:?}", r.kind));
    }
    panic!("the stack never emptied");
}

fn lives(g: &Game) -> (i32, i32) {
    (g.state.player(YOU).life, g.state.player(THEM).life)
}

/// The question says each mode's pawprints and the budget, and allows none (CR 700.2i).
#[test]
fn the_question_counts_pawprints() {
    let (mut g, season, _) = game();
    cast(&mut g, season);
    let r = next(&mut g);
    let RequestKind::ChooseModes { pawprints, at_most, count, repeat, eligible, .. } = &r.kind else { panic!("got {:?}", r.kind) };
    assert_eq!((pawprints.as_slice(), *at_most, *count, *repeat), (&[1, 2, 3][..], 5, 0, true));
    assert_eq!(eligible, &[0, 1, 2]);
}

/// The same mode three times runs three times; one {P} of five is fewer than five, and legal.
#[test]
fn the_same_mode_three_times_or_once() {
    for (chosen, gained) in [(vec![0, 0, 0], 3), (vec![0], 1), (vec![0, 0, 0, 0, 0], 5)] {
        let (mut g, season, _) = game();
        let (you, _) = lives(&g);
        cast_season(&mut g, season, &chosen, &[]);
        resolve(&mut g);
        assert_eq!(lives(&g).0, you + gained, "{chosen:?}");
    }
}

/// {P}{P}{P} and {P}{P}: five, each once; and the {P} before the {P}{P}{P}, in printed order whichever order they were named in.
#[test]
fn a_three_and_two_split_runs_in_printed_order() {
    let (mut g, season, _) = game();
    let (you, them) = lives(&g);
    cast_season(&mut g, season, &[2, 1], &[THEM]);
    resolve(&mut g);
    assert_eq!(lives(&g), (you * 2, them - 2));

    let (mut g, season, _) = game();
    cast_season(&mut g, season, &[2, 0], &[]);
    resolve(&mut g);
    assert_eq!(lives(&g).0, (you + 1) * 2, "gain 1, then double");
}

/// No modes at all is a choice (CR 700.2i): the spell is cast, resolves and does nothing.
#[test]
fn zero_modes_is_legal() {
    let (mut g, season, _) = game();
    let before = lives(&g);
    cast_season(&mut g, season, &[], &[]);
    assert_eq!(g.state.stack.len(), 1, "it was cast");
    resolve(&mut g);
    assert_eq!(lives(&g), before);
    assert_eq!(g.state.zone(Zone::graveyard(YOU)).len(), 1, "it resolved to the graveyard");
}

/// With no mode it could choose (its only one has no creature to target, CR 601.2c), a pawprint spell is still cast, with
/// none (CR 700.2i: any number of modes, zero too): the choices it could be cast with hold the empty one.
#[test]
fn with_no_mode_it_could_choose_it_is_still_cast_with_none() {
    let table: &'static CardTable = Box::leak(Box::new(CardTable::new([&FILLER, &LONE])));
    let (mut g, hand) = game_with(table, &[&LONE]);
    let before = lives(&g);
    cast_season(&mut g, hand[0], &[], &[]);
    assert_eq!(g.state.stack.len(), 1, "it was cast");
    resolve(&mut g);
    assert_eq!(lives(&g), before);
    assert_eq!(g.state.zone(Zone::graveyard(YOU)).len(), 1, "it resolved to the graveyard");
}

/// More pawprints than five is refused, however they are made up.
#[test]
fn more_than_five_pawprints_is_refused() {
    for chosen in [vec![2, 2], vec![1, 1, 1], vec![0, 0, 0, 0, 0, 0], vec![0, 1, 2]] {
        let (mut g, season, _) = game();
        cast(&mut g, season);
        let r = next(&mut g);
        assert!(g.answer(r.id, r.player, Answer::Modes(chosen.clone())).is_err(), "{chosen:?} is refused");
    }
}

/// A mode chosen twice has a target for each instance (CR 700.2d): one aimed at each player.
#[test]
fn each_instance_has_its_own_target() {
    let (mut g, season, _) = game();
    let (you, them) = lives(&g);
    cast_season(&mut g, season, &[1, 1, 0], &[YOU, THEM]);
    resolve(&mut g);
    assert_eq!(lives(&g), (you - 2 + 1, them - 2));
}

/// A copy of the Season has its modes (CR 707.10), and its targets may change (CR 707.10c): the copy's {P}{P} is aimed at you
/// instead, while the Season's own still hits them.
#[test]
fn a_copy_keeps_its_modes_and_may_change_its_targets() {
    let (mut g, season, twin) = game();
    let (you, them) = lives(&g);
    cast_season(&mut g, season, &[1, 0], &[THEM]);
    cast(&mut g, twin);
    let mut changed = false;
    for _ in 0..20 {
        let r = next(&mut g);
        match &r.kind {
            RequestKind::PayCosts { options, .. } => {
                let pay = options.iter().find(|o| matches!(o.kind, ActionKind::ConfirmCasting | ActionKind::AssignMana)).expect("it pays").clone();
                g.answer(r.id, r.player, Answer::Act(Action::of(&pay))).expect("a legal answer");
            }
            RequestKind::May { .. } => g.answer(r.id, r.player, Answer::Yes).expect("an answer"),
            RequestKind::ChooseTarget { candidates, .. } if candidates.contains(&Target::Player(YOU)) => {
                changed = true;
                g.answer(r.id, r.player, Answer::Targets(vec![Target::Player(YOU)])).expect("a new target");
            }
            _ if g.state.stack.is_empty() => break,
            _ => g.answer(r.id, r.player, Answer::Pass).unwrap_or_else(|e| panic!("{e:?} for {:?}", r.kind)),
        }
    }
    assert!(changed, "the copy's target was asked again");
    assert!(g.state.stack.is_empty());
    // The copy: you gain 1 and lose 2; the Season: you gain 1, they lose 2.
    assert_eq!(lives(&g), (you + 1 - 2 + 1, them - 2));
}
