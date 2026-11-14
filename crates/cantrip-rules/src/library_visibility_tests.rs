//! Continuous visibility, announcement boundaries, and CR 401.6 identity.
use crate::{library_visibility as visibility, ops, Cx, Game, Setup};
use cantrip_api::*;
use cantrip_state::{Frame, FrameKind};

const YOU: PlayerId = PlayerId(0);
const THEM: PlayerId = PlayerId(1);

// The same printed rule on Benjamin Sisko, Besieged and Garruk's Horde.
static SISKO: CardDef = CardDef {
    name: "Benjamin Sisko, Besieged", types: TypeSet::of(&[CardType::Creature]),
    power: Some(5), toughness: Some(5), play_rules: &[PlayRule::RevealLibraryTop, PlayRule::LibraryTopLands], ..CardDef::BLANK
};
static HORDE: CardDef = CardDef {
    name: "Garruk's Horde", types: TypeSet::of(&[CardType::Creature]),
    power: Some(7), toughness: Some(7), play_rules: &[PlayRule::RevealLibraryTop], ..CardDef::BLANK
};
static A: CardDef = CardDef { name: "Library A", types: TypeSet::of(&[CardType::Creature]), ..CardDef::BLANK };
static B: CardDef = CardDef { name: "Library B", types: TypeSet::of(&[CardType::Land]), ..CardDef::BLANK };
static C: CardDef = CardDef { name: "Library C", types: TypeSet::of(&[CardType::Instant]), ..CardDef::BLANK };

fn cx(game: &mut Game) -> Cx<'_> {
    Cx { g: game, frame: usize::MAX, cursor: 0, mana_source: None, entering: None,
        lineage: Vec::new(), damage: None, asked_at: None, doer: None, producing: None }
}

fn put(game: &mut Game, def: &'static CardDef, zone: Zone) -> ObjectId {
    let id = game.state.push_obj(Obj::new(game.cards.id_of(def), zone.player, zone));
    game.state.zone_push(zone, id);
    game.state.see_statics();
    id
}

fn game() -> (Game, [ObjectId; 3]) {
    let cards = Box::leak(Box::new(CardTable::new([&SISKO, &HORDE, &A, &B, &C])));
    let mut game = Game::new(cards, &Setup { decks: vec![vec![cards.id_of(&A); 7]; 3], ..Setup::default() });
    let ids = [&A, &B, &C].map(|def| put(&mut game, def, Zone::library(YOU)));
    put(&mut game, &C, Zone::library(THEM));
    (game, ids)
}

fn source(game: &mut Game, def: &'static CardDef) -> ObjectId {
    let id = put(game, def, Zone::BATTLEFIELD);
    visibility::sync(&mut cx(game));
    id
}

#[test]
fn both_printed_callers_reveal_only_their_current_top_and_handle_empty_libraries() {
    for def in [&SISKO, &HORDE] {
        let (mut game, [a, b, c]) = game();
        assert!(game.revealed_in_hidden_zones().is_empty());
        source(&mut game, def);
        assert_eq!(game.revealed_in_hidden_zones(), [a]);
        ops::draw(&mut cx(&mut game), YOU);
        assert_eq!(game.revealed_in_hidden_zones(), [b]);
        ops::draw(&mut cx(&mut game), YOU);
        assert_eq!(game.revealed_in_hidden_zones(), [c]);
        ops::draw(&mut cx(&mut game), YOU);
        assert_eq!(game.visible_library_top(YOU), None);
        assert!(game.revealed_in_hidden_zones().is_empty());
        assert!(!game.state.player(YOU).drew_from_empty);
    }
}

#[test]
fn losing_one_of_two_sources_preserves_identity_and_the_last_source_loss_renews_it() {
    let (mut game, [a, _, _]) = game();
    let first = source(&mut game, &SISKO);
    let second = source(&mut game, &HORDE);
    ops::sacrifice(&mut cx(&mut game), YOU, &[first]);
    assert_eq!(game.revealed_in_hidden_zones(), [a]);
    let mark = game.state.mark();
    let before = game.state.clone();
    let log = game.log.len();
    ops::sacrifice(&mut cx(&mut game), YOU, &[second]);
    assert!(game.revealed_in_hidden_zones().is_empty());
    let renewed = game.state.zone(Zone::library(YOU))[0];
    assert_ne!(a, renewed, "CR 401.6 concealing the card creates a new object");
    assert_eq!(game.state.obj(a).zone, Zone::GONE);
    assert!(game.state.obj(a).last_known.is_some());
    assert_eq!(cx(&mut game).chars(a).name, "Library A", "LKI remains available");
    assert_eq!(game.state.obj(a).became, None, "a stale reference must not follow the new object");
    assert!(!game.log[log..].iter().any(|r| matches!(r.event, Event::ZoneChange { old, .. } if old == a)));
    game.state.rollback(mark);
    game.log.truncate(log);
    assert!(game.state == before, "visibility and renewed identity must be journalled");
    assert_eq!(game.revealed_in_hidden_zones(), [a]);
}

#[test]
fn control_transfer_changes_which_library_is_revealed_immediately() {
    let (mut game, [a, _, _]) = game();
    let sisko = source(&mut game, &SISKO);
    let other = game.state.zone(Zone::library(THEM))[0];
    cantrip_api::Game::add_effect(&mut cx(&mut game), sisko, sisko, EffectKind::Control(THEM), Duration::Indefinite);
    assert_eq!(game.state.obj(sisko).controller, THEM);
    assert_eq!(game.revealed_in_hidden_zones(), [other]);
    assert_ne!(game.state.zone(Zone::library(YOU))[0], a);
}

#[test]
fn a_buried_revealed_card_is_renewed_without_a_zone_change_and_returns_as_a_new_object() {
    let (mut game, [a, b, _]) = game();
    source(&mut game, &SISKO);
    assert!(ops::seen_by_all(&cx(&mut game), a, false), "rules look-back must see the revealed top as public");
    let log = game.log.len();
    ops::arrange_library(&mut cx(&mut game), YOU, &[b], &[]);
    assert_eq!(game.revealed_in_hidden_zones(), [b]);
    let renewed_a = game.state.zone(Zone::library(YOU))[1];
    assert_ne!(renewed_a, a);
    assert!(!ops::seen_by_all(&cx(&mut game), renewed_a, false), "a buried card is hidden again");
    assert!(game.state.obj(a).last_known.is_some());
    assert!(!game.log[log..].iter().any(|r| matches!(r.event, Event::ZoneChange { .. })));
    ops::arrange_library(&mut cx(&mut game), YOU, &[renewed_a], &[]);
    assert_eq!(game.revealed_in_hidden_zones(), [renewed_a]);
}

#[test]
fn each_successive_top_is_revealed_before_the_next_draw_with_a_rules_cause() {
    let (mut game, [a, b, c]) = game();
    source(&mut game, &HORDE);
    ops::draw(&mut cx(&mut game), YOU);
    ops::draw(&mut cx(&mut game), YOU);
    ops::draw(&mut cx(&mut game), YOU);
    for (top, position) in [(a, 0), (b, 1), (c, 2)] {
        let reveal = game.log.iter().find(|r| matches!(r.event, Event::Revealed { card, .. } if card == top)).unwrap();
        let draw = game.log.iter().filter(|r| matches!(r.event, Event::Drew { .. })).nth(position).unwrap();
        assert!(reveal.serial < draw.serial, "each new top is visible before its draw");
        assert_eq!(reveal.cause, cantrip_api::history::Cause::Rules);
    }
}

#[test]
fn reordered_revealed_selection_renews_even_when_same_card_finishes_on_top() {
    let (mut game, [a, b, c]) = game();
    source(&mut game, &SISKO);
    let before = game.state.clone();
    let mark = game.state.mark();
    let from = game.log.len();
    // Sensei's Divining Top and Ponder use this API to put their selection back.
    ops::arrange_library(&mut cx(&mut game), YOU, &[a, c, b], &[]);
    let renewed = game.state.zone(Zone::library(YOU))[0];
    assert_ne!(renewed, a, "CR 701.20d: a revealed card in the reordered selection becomes a new object even when it finishes on top");
    assert_eq!(game.state.obj(renewed).card, game.cards.id_of(&A));
    assert_eq!(game.state.zone(Zone::library(YOU))[1..], [c, b]);
    assert_eq!(game.revealed_in_hidden_zones(), [renewed]);
    assert_eq!(game.state.obj(a).zone, Zone::GONE);
    assert!(game.state.obj(a).last_known.is_some());
    assert_eq!(cx(&mut game).chars(a).name, "Library A", "old references retain LKI");
    assert_eq!(game.state.obj(a).became, None, "stale references do not follow the new object");
    assert!(!game.log[from..].iter().any(|r| matches!(r.event, Event::ZoneChange { .. })), "renewal is not a zone change");
    assert!(game.log[from..].iter().any(|r| matches!(r.event, Event::Revealed { card, .. } if card == renewed)
        && r.cause == cantrip_api::history::Cause::Rules));
    game.state.rollback(mark);
    game.log.truncate(from);
    assert!(game.state == before, "ordering, visibility, identity and LKI roll back together");
    assert_eq!(game.revealed_in_hidden_zones(), [a]);
}

#[test]
fn reordering_only_hidden_cards_preserves_the_revealed_top_identity() {
    let (mut game, [a, b, c]) = game();
    source(&mut game, &SISKO);
    let from = game.log.len();
    ops::arrange_library(&mut cx(&mut game), YOU, &[], &[c, b]);
    assert_eq!(game.state.zone(Zone::library(YOU)), [a, c, b]);
    assert_eq!(game.revealed_in_hidden_zones(), [a]);
    assert!(game.log[from..].is_empty(), "hidden reordering does not interrupt the top's reveal");
    ops::arrange_library(&mut cx(&mut game), YOU, &[], &[]);
    assert_eq!(game.revealed_in_hidden_zones(), [a]);
}

#[test]
fn reordering_without_a_reveal_preserves_every_object_identity() {
    let (mut game, [a, b, c]) = game();
    ops::arrange_library(&mut cx(&mut game), YOU, &[a, c, b], &[]);
    assert_eq!(game.state.zone(Zone::library(YOU)), [a, c, b]);
    assert!(game.revealed_in_hidden_zones().is_empty());
    assert!(game.state.obj(a).last_known.is_none());
}

#[test]
fn insertion_and_ordering_never_reveal_a_provisional_top() {
    for place in [LibraryPlace::Top, LibraryPlace::Bottom, LibraryPlace::FromTop(1)] {
        let (mut game, [a, _, _]) = game();
        source(&mut game, &SISKO);
        let b = put(&mut game, &B, Zone::hand(YOU));
        let c = put(&mut game, &C, Zone::hand(YOU));
        let log = game.log.len();
        let moved = ops::put_into_library(&mut cx(&mut game), &[b, c], place);
        let reveals: Vec<_> = game.log[log..].iter().filter_map(|r| match r.event { Event::Revealed { card, .. } => Some(card), _ => None }).collect();
        if place == LibraryPlace::Top {
            assert_eq!(reveals, [moved[0]], "only the final arranged top is revealed");
            assert_eq!(game.revealed_in_hidden_zones(), [moved[0]]);
        } else {
            assert!(reveals.is_empty(), "a bottom/interior insertion cannot reveal its temporary top");
            assert_eq!(game.revealed_in_hidden_zones(), [a]);
        }
    }
}

#[test]
fn even_a_single_card_shuffle_interrupts_the_reveal_and_records_the_new_identity() {
    let (mut game, [a, b, c]) = game();
    ops::put_into_graveyard(&mut cx(&mut game), &[b, c]);
    source(&mut game, &HORDE);
    crate::turn::shuffle_library(&mut cx(&mut game), YOU);
    let new = game.state.zone(Zone::library(YOU))[0];
    assert_ne!(a, new);
    assert_eq!(game.revealed_in_hidden_zones(), [new]);
    assert!(matches!(game.log[game.log.len()-2].event, Event::Shuffled { .. }));
    assert!(matches!(game.log.last().unwrap().event, Event::Revealed { card, .. } if card == new));
}

#[test]
fn announcement_holds_unchanged_tops_but_conceals_changed_tops_until_completion() {
    let (mut game, [a, b, _]) = game();
    source(&mut game, &SISKO);
    game.state.frame_push(Frame::new(FrameKind::Act { player: YOU }));
    let frame = game.state.agenda.len()-1;
    let mut cx = cx(&mut game);
    cx.frame = frame;
    visibility::hold(&mut cx);
    assert_eq!(cx.g.revealed_in_hidden_zones(), [a], "an unchanged top stays visible");
    ops::move_to(&mut cx, a, Zone::STACK);
    assert_eq!(cx.g.visible_library_top(YOU), None);
    assert!(cx.g.revealed_in_hidden_zones().is_empty(), "casting, activation and special actions conceal a new top");
    visibility::release(&mut cx);
    assert_eq!(cx.g.revealed_in_hidden_zones(), [b]);
}

#[test]
fn nested_announcements_cannot_release_the_outer_casting_barrier() {
    let (mut game, [a, b, _]) = game();
    source(&mut game, &SISKO);
    for _ in 0..2 { game.state.frame_push(Frame::new(FrameKind::Act { player: YOU })); }
    let mut cx = cx(&mut game);
    cx.frame = 0;
    visibility::hold(&mut cx);
    cx.frame = 1;
    visibility::hold(&mut cx);
    ops::draw(&mut cx, YOU);
    visibility::release(&mut cx);
    assert_eq!(cx.g.visible_library_top(YOU), None);
    cx.frame = 0;
    visibility::release(&mut cx);
    assert_eq!(cx.g.revealed_in_hidden_zones(), [b]);
    assert_ne!(a, b);
}

#[test]
fn goto_undoes_and_redoes_a_library_land_play_and_its_visibility_without_replaying() {
    let (mut game, _) = game();
    source(&mut game, &SISKO);
    game.verify = true;
    game.run();
    for _ in 0..40 {
        if game.state.turn.step() == Step::PrecombatMain && game.pending().is_some_and(|r| r.player == YOU) { break; }
        let request = game.pending().cloned().expect("priority");
        game.answer(request.id, request.player, Answer::Pass).expect("pass");
    }
    let request = game.pending().cloned().expect("main phase");
    let RequestKind::Priority { options } = &request.kind else { panic!("priority") };
    let land = options.iter().find(|o| o.kind == ActionKind::PlayLand && o.source.is_some_and(|id| game.state.obj(id).zone == Zone::library(YOU))).expect("the revealed land can be played");
    let answer = Answer::Act(Action::of(land));
    let root = game.checkpoint();
    let before = game.state.clone();
    let before_log = game.log.clone();
    game.answer(request.id, YOU, answer.clone()).expect("play");
    let child = game.checkpoint();
    let after = game.state.clone();
    let after_log = game.log.clone();
    assert_ne!(before.player(YOU).revealed_library_top, after.player(YOU).revealed_library_top);
    game.goto(root);
    assert!(game.state == before && game.log == before_log);
    game.goto(child);
    assert!(game.state == after && game.log == after_log);
    game.goto(root);
    game.answer(request.id, YOU, answer).expect("same answer follows the existing child");
    assert_eq!(game.checkpoint(), child);
    assert!(game.state == after && game.log == after_log);
}

// A test-only mana ability that gains life. Under CR 605.1a replacement effects
// do not affect its classification: a Lich-type replacement can draw cards as it
// resolves. Its own text neither moves a library card nor asks for a target.
fn life_mana(g: &mut dyn cantrip_api::Game, a: &Asker, _: Option<Color>) -> Result<(), Suspend> {
    g.add_mana(a.controller, Mana::C, 1);
    g.gain_life(a.controller, 2);
    Ok(())
}

fn replace_gain(_: &dyn Query, a: &Asker, p: &Proposal) -> bool {
    matches!(p, Proposal::LifeChange(l) if l.player == a.controller && l.amount > 0)
}
fn no_gain(_: &dyn Query, _: &Asker, p: &mut Proposal) {
    if let Proposal::LifeChange(l) = p { l.amount = 0; }
}
fn draw_instead(g: &mut dyn cantrip_api::Game, _: &Asker, before: &Proposal, _: &Proposal) {
    if let Proposal::LifeChange(l) = before { g.draw(l.player, l.amount as u32); }
}
fn nothing(_: &mut dyn cantrip_api::Game, _: &StackItem) -> Result<(), Suspend> { Ok(()) }
fn creature(q: &dyn Query, _: &Asker, id: ObjectId) -> bool { q.chars(id).is(CardType::Creature) }

static LIFE_MANA: CardDef = CardDef {
    name: "Visibility Test Life Mana", types: TypeSet::of(&[CardType::Artifact]),
    abilities: &[mana_ability(TAP, add(Mana::C).by(life_mana), "{T}: Add {C}. You gain 2 life.")],
    ..CardDef::BLANK
};
static PAYING_MANA: CardDef = CardDef {
    name: "Visibility Test Paying Mana",
    abilities: &[mana_ability(TAP.and(cost("{1}")), add(Mana::C).by(life_mana), "{1}, {T}: Add {C}. You gain 2 life.")], ..LIFE_MANA
};
static PLAIN_MANA: CardDef = CardDef {
    name: "Visibility Test Plain Mana", types: TypeSet::of(&[CardType::Artifact]),
    abilities: &[mana_ability(TAP, add(Mana::C), "{T}: Add {C}.")], ..CardDef::BLANK
};
static LIFE_TO_DRAWS: CardDef = CardDef {
    name: "Visibility Test Life Replacement", types: TypeSet::of(&[CardType::Enchantment]),
    replacements: &[replacement(ReplacementKind::LifeChange, replace_gain, no_gain, "Draw instead of gaining life").then(draw_instead)],
    ..CardDef::BLANK
};
static TOP_CREATURE: CardDef = CardDef {
    name: "Visibility Test Creature", cost: mana("{2}"), has_cost: true,
    types: TypeSet::of(&[CardType::Creature]), power: Some(2), toughness: Some(2), ..CardDef::BLANK
};
static TOP_PERMISSION: CardDef = CardDef {
    play_rules: &[PlayRule::RevealLibraryTop, PlayRule::LibraryTopSpells(creature, None)], ..SISKO
};
static OUTER_ABILITY: CardDef = CardDef {
    name: "Visibility Test Outer Ability", types: TypeSet::of(&[CardType::Artifact]),
    abilities: &[ability(cost("{2}"), nothing, "{2}: Do nothing.")], ..CardDef::BLANK
};

fn land_arrives(q: &dyn Query, _: ObjectId, event: &Event) -> Option<Fired> {
    if matches!(event, Event::ZoneChange { to, reason: ZoneReason::PlayLand, .. } if *to == Zone::BATTLEFIELD) {
        assert_eq!(q.state().player(YOU).revealed_library_top, None,
            "a land play during resolution is still a special action: its changed top waits until completion");
    }
    None
}
static LAND_OBSERVER: CardDef = CardDef {
    name: "Visibility Test Land Observer", types: TypeSet::of(&[CardType::Artifact]),
    abilities: &[triggered(when(EventMask::ZONE_CHANGE, land_arrives), nothing, "Whenever a land is played, observe its timing.")], ..CardDef::BLANK
};

fn mana_game(casting: bool, paid: bool) -> (Game, ObjectId, [ObjectId; 3], ObjectId) {
    let cards = Box::leak(Box::new(CardTable::new([&TOP_PERMISSION, &TOP_CREATURE, &LIFE_MANA, &PAYING_MANA, &LIFE_TO_DRAWS, &OUTER_ABILITY, &LAND_OBSERVER, &PLAIN_MANA, &A, &B, &C])));
    let mut game = Game::new(cards, &Setup { decks: vec![vec![cards.id_of(&A); 7]; 2], ..Setup::default() });
    let tops = [&A, &B, &C].map(|d| put(&mut game, d, Zone::library(YOU)));
    source(&mut game, &TOP_PERMISSION);
    source(&mut game, &LIFE_TO_DRAWS);
    let mana = source(&mut game, if paid { &PAYING_MANA } else { &LIFE_MANA });
    source(&mut game, &PLAIN_MANA);
    let outer = if casting {
        let spell = put(&mut game, &TOP_CREATURE, Zone::library(YOU));
        ops::arrange_library(&mut cx(&mut game), YOU, &[spell], &[]);
        spell
    } else { source(&mut game, &OUTER_ABILITY) };
    game.verify = true;
    game.run();
    for _ in 0..40 {
        if game.state.turn.step() == Step::PrecombatMain && game.pending().is_some_and(|r| r.player == YOU) { return (game, mana, tops, outer); }
        let r = game.pending().cloned().expect("priority before main");
        game.answer(r.id, r.player, Answer::Pass).unwrap();
    }
    panic!("main phase");
}

fn choose_action(game: &mut Game, kind: ActionKind, source: ObjectId) {
    let r = game.pending().cloned().expect("question");
    let options = match &r.kind { RequestKind::Priority { options } | RequestKind::PayCosts { options, .. } => options, other => panic!("{other:?}") };
    let option = options.iter().find(|o| o.kind == kind && o.source == Some(source)).expect("offered action");
    game.answer(r.id, r.player, Answer::Act(Action::of(option))).unwrap();
}

#[test]
fn standalone_mana_activation_reveals_successive_draws_during_its_resolution() {
    for paid in [false, true] {
        let (mut game, mana, [a, b, c], _) = mana_game(false, paid);
        let from = game.log.len();
        choose_action(&mut game, ActionKind::ActivateAbility, mana);
        if paid {
            assert_eq!(game.revealed_in_hidden_zones(), [a], "unchanged top remains visible while activation waits for its own mana cost");
            let plain = game.state.battlefield().iter().copied().find(|&id| game.state.obj(id).card == game.cards.id_of(&PLAIN_MANA)).unwrap();
            choose_action(&mut game, ActionKind::ActivateAbility, plain);
        }
        assert_eq!(game.revealed_in_hidden_zones(), [c]);
        assert_eq!(game.state.player(YOU).life, 20, "the gain was replaced, not triggered");
        let draw_b = game.log[from..].iter().find(|r| matches!(r.event, Event::Drew { card, .. } if card == b || game.state.obj(card).card == game.cards.id_of(&B))).expect("second draw");
        let reveal_b = game.log[from..].iter().find(|r| matches!(r.event, Event::Revealed { card, .. } if card == b)).expect("second top revealed during mana resolution");
        assert!(reveal_b.serial < draw_b.serial);
        assert!(game.log[from..].iter().any(|r| matches!(r.event, Event::ZoneChange { old, .. } if old == a)));
    }
}

#[test]
fn mana_activation_used_for_an_outer_announcement_keeps_all_new_tops_hidden() {
    for casting in [true, false] {
        let (mut game, mana, [_, b, c], outer) = mana_game(casting, false);
        choose_action(&mut game, if casting { ActionKind::CastSpell } else { ActionKind::ActivateAbility }, outer);
        let from = game.log.len();
        choose_action(&mut game, ActionKind::ActivateAbility, mana);
        assert_eq!(game.visible_library_top(YOU), None, "outer announcement still in progress");
        assert!(!game.log[from..].iter().any(|r| matches!(r.event, Event::Revealed { card, .. } if card == b || card == c)), "no intermediate new top is revealed");
        assert_eq!(game.state.zone(Zone::library(YOU)), [c]);
        let plain = game.state.battlefield().iter().copied().find(|&id| game.state.obj(id).card == game.cards.id_of(&PLAIN_MANA)).unwrap();
        choose_action(&mut game, ActionKind::ActivateAbility, plain);
        if let Some(RequestKind::PayCosts { options, .. }) = game.pending().map(|r| &r.kind) {
            let confirm = options.iter().find(|o| o.kind == ActionKind::ConfirmCasting).expect("confirm").clone();
            let r = game.pending().cloned().unwrap();
            game.answer(r.id, YOU, Answer::Act(Action::of(&confirm))).unwrap();
        }
        assert_eq!(game.revealed_in_hidden_zones(), [c]);
        assert_eq!(game.log[from..].iter().filter(|r| matches!(r.event, Event::Drew { .. })).count(), 2, "both hidden cards were actually drawn");
    }
}

#[test]
fn a_land_play_during_resolution_hides_its_changed_top_until_the_special_action_finishes() {
    let (mut game, _, [_, land, _], outer) = mana_game(false, false);
    ops::arrange_library(&mut cx(&mut game), YOU, &[land], &[]);
    source(&mut game, &LAND_OBSERVER);
    let item = crate::actions::stack_item(StackKind::Activated, outer, AbilityRef { card: game.cards.id_of(&OUTER_ABILITY), index: 0 }, YOU, &[]);
    game.state.stack_push(item);
    let serial = game.state.stack.last().unwrap().serial;
    game.state.frame_push(Frame::new(FrameKind::Resolve { serial }));
    let frame = game.state.agenda.len()-1;
    let mut cx = cx(&mut game);
    cx.frame = frame;
    assert_eq!(cx.g.revealed_in_hidden_zones(), [land]);
    let from = cx.g.log.len();
    let moved = crate::actions::play_land_during_resolution(&mut cx, land).unwrap().expect("land played");
    assert_eq!(cx.s().obj(moved).zone, Zone::BATTLEFIELD);
    assert_eq!(cx.s().player(YOU).lands_played, 1);
    let top = cx.s().zone(Zone::library(YOU))[0];
    assert_eq!(cx.g.revealed_in_hidden_zones(), [top], "new top is visible before the effect continues");
    let played = cx.g.log[from..].iter().find(|r| matches!(r.event, Event::LandPlayed { .. })).unwrap();
    let revealed = cx.g.log[from..].iter().find(|r| matches!(r.event, Event::Revealed { card, .. } if card == top)).unwrap();
    assert!(played.serial < revealed.serial, "the special action completes before revealing its new top");
}

#[test]
fn effect_reveal_scopes_nest_and_a_shuffle_hides_the_old_identity() {
    use cantrip_api::Game as _;
    let (mut g, ids) = game();
    while !g.state.agenda.is_empty() { g.state.frame_remove(0); }
    g.state.frame_push(Frame::new(FrameKind::BeginStep));
    let card = ids[0];
    fn scope(game: &mut Game) -> Cx<'_> { Cx { g: game, frame: 0, cursor: 0, mana_source: None, entering: None,
        lineage: Vec::new(), damage: None, asked_at: None, doer: None, producing: None } }
    scope(&mut g).begin_effect_reveal(YOU, &[card]);
    scope(&mut g).begin_effect_reveal(YOU, &[card]);
    assert!(g.revealed_in_hidden_zones().contains(&card));
    let resumed_at = g.log.len() as u32;
    g.state.update_frame(0, |frame| { frame.staged = true; frame.log_mark = resumed_at; });
    assert!(g.revealed_in_hidden_zones().contains(&card), "a staged continuation retains its reveal start");
    let mut child = Frame::new(FrameKind::Priority);
    child.log_mark = g.log.len() as u32;
    g.state.frame_push(child);
    assert!(g.revealed_in_hidden_zones().contains(&card), "a nested frame retains the outer reveal");
    g.state.frame_remove(g.state.agenda.len() - 1);
    scope(&mut g).end_effect_reveal(&[card]);
    assert!(g.revealed_in_hidden_zones().contains(&card), "an outer reveal remains open");
    let original_card = g.state.obj(card).card;
    cx(&mut g).shuffle(YOU);
    assert!(!g.revealed_in_hidden_zones().contains(&card), "the shuffled identity is no longer publicly trackable");
    assert_eq!(g.state.obj(card).zone, Zone::GONE, "a shuffled revealed card becomes a new object");
    assert!(g.state.zone(Zone::library(YOU)).iter().any(|&id| id != card && g.state.obj(id).card == original_card));
    scope(&mut g).end_effect_reveal(&[card]);
    assert!(!g.revealed_in_hidden_zones().contains(&card));
}

#[test]
fn reordered_effect_revealed_card_is_renewed_and_undo_restores_scope() {
    use cantrip_api::Game as _;
    let (mut g, [a, b, _]) = game();
    while !g.state.agenda.is_empty() { g.state.frame_remove(0); }
    g.state.frame_push(Frame::new(FrameKind::BeginStep));
    let mut scope = Cx { g: &mut g, frame: 0, cursor: 0, mana_source: None, entering: None,
        lineage: Vec::new(), damage: None, asked_at: None, doer: None, producing: None };
    scope.begin_effect_reveal(YOU, &[a]);
    let before = scope.g.state.clone();
    let mark = scope.g.state.mark();
    let log = scope.g.log.len();
    ops::arrange_library(&mut scope, YOU, &[], &[a]);
    let renewed = *scope.g.state.zone(Zone::library(YOU)).last().unwrap();
    assert_ne!(a, renewed);
    assert_eq!(scope.g.state.obj(a).zone, Zone::GONE);
    assert_eq!(scope.g.state.obj(a).became, None);
    assert!(!scope.g.revealed_in_hidden_zones().contains(&renewed));
    assert!(scope.g.state.zone(Zone::library(YOU)).contains(&b));
    scope.g.state.rollback(mark);
    scope.g.log.truncate(log);
    assert!(scope.g.state == before);
    assert!(scope.g.revealed_in_hidden_zones().contains(&a));
}
