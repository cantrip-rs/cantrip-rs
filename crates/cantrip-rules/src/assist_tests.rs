//! Assist as the core does it (CR 702.132a), over test cards: with a spell whose total cost has generic mana, before any
//! mana ability is activated while casting it, its caster may choose another player; that player has a chance to activate
//! mana abilities, then the caster does; then the chosen player may pay any amount of the generic mana - and none of the
//! colored (the ruling) - from their own pool, and the caster pays the rest, the mana abilities over for both (CR 601.2g).

use crate::{Game, Setup};
use cantrip_api::*;
use cantrip_state::{Frame, FrameKind};

const YOU: PlayerId = PlayerId(0);
const THEM: PlayerId = PlayerId(1);

static FILLER: CardDef = CardDef { name: "Assist Test Filler", has_cost: false, types: TypeSet::of(&[CardType::Land]), ..CardDef::BLANK };

/// "{T}: Add {G}."
static FOREST: CardDef = CardDef {
    name: "Assist Test Forest",
    has_cost: false,
    types: TypeSet::of(&[CardType::Land]),
    supertypes: Supertypes { basic: true, legendary: false, snow: false },
    subtypes: SubtypeSet::of(&[Subtype::Forest]),
    basic_land_types: &[Mana::G],
    ..CardDef::BLANK
};

/// "Assist / Trample": a 7/5 for {5}{G}.
static BINOX: CardDef = CardDef {
    name: "Assist Test Binox",
    cost: mana("{5}{G}"),
    types: TypeSet::of(&[CardType::Creature]),
    keywords: Keywords::of(&[Keyword::Trample]),
    power: Some(7),
    toughness: Some(5),
    pay_with: PayWith { assist: true, ..PayWith::NONE },
    ..CardDef::BLANK
};

/// The same creature without assist.
static PLAIN_BINOX: CardDef = CardDef { name: "Assist Test Plain Binox", pay_with: PayWith::NONE, ..BINOX };

fn game() -> Game {
    let table: &'static CardTable = Box::leak(Box::new(CardTable::new([&FILLER, &FOREST, &BINOX, &PLAIN_BINOX])));
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
    g
}

fn put(g: &mut Game, def: &'static CardDef, owner: PlayerId, zone: Zone) -> ObjectId {
    let id = g.state.push_obj(Obj::new(g.cards.id_of(def), owner, zone));
    g.state.zone_push(zone, id);
    id
}

fn add(g: &mut Game, player: PlayerId, mana: Mana, n: u32) {
    let source = g.state.zone(Zone::library(player))[0];
    g.state.add_mana(player, mana, n, source, None, ManaLasts::Step);
    if let Some(at) = g.state.agenda.len().checked_sub(1)
        && g.state.agenda[at].pending.as_ref().is_some_and(|r| matches!(r.kind, RequestKind::Priority { .. }))
    {
        g.state.update_frame(at, |f| f.pending = None);
    }
}

fn next(g: &mut Game) -> Request {
    g.run();
    g.pending().cloned().expect("a question")
}

fn cast_option(g: &mut Game, card: ObjectId) -> Option<ActionOption> {
    match next(g).kind {
        RequestKind::Priority { options } => options.into_iter().find(|o| o.kind == ActionKind::CastSpell && o.source == Some(card) && o.index == 0),
        other => panic!("expected priority, got {other:?}"),
    }
}

fn answer(g: &mut Game, a: Answer) {
    let r = next(g);
    g.answer(r.id, r.player, a).expect("a legal answer");
}

fn act(g: &mut Game, kind: ActionKind) {
    let r = next(g);
    let options = match &r.kind {
        RequestKind::PayCosts { options, .. } => options.clone(),
        other => panic!("expected a payment, got {other:?}"),
    };
    let o = options.iter().find(|o| o.kind == kind).unwrap_or_else(|| panic!("{kind:?} offered in {options:?}")).clone();
    g.answer(r.id, r.player, Answer::Act(Action::of(&o))).expect("a legal answer");
}

/// Pays what the asked player owes from their pool, until priority comes back.
fn pay_rest(g: &mut Game) {
    loop {
        let r = next(g);
        let RequestKind::PayCosts { options, .. } = &r.kind else { return };
        let pay = options.iter().find(|o| matches!(o.kind, ActionKind::ConfirmCasting | ActionKind::AssignMana)).expect("the pool pays").clone();
        g.answer(r.id, r.player, Answer::Act(Action::of(&pay))).expect("a legal answer");
    }
}

/// CR 702.132a: the cast is offered where the caster's mana and another player's together pay it, and not one mana short;
/// a card without assist is not.
#[test]
fn assist_makes_a_cast_affordable_with_another_players_mana() {
    let mut g = game();
    let card = put(&mut g, &BINOX, YOU, Zone::hand(YOU));
    let plain = put(&mut g, &PLAIN_BINOX, YOU, Zone::hand(YOU));
    add(&mut g, YOU, Mana::G, 1);
    add(&mut g, YOU, Mana::C, 1);
    add(&mut g, THEM, Mana::C, 3);
    assert!(cast_option(&mut g, card).is_none(), "{{G}}{{1}} and their {{3}} are one short of {{5}}{{G}}");
    add(&mut g, YOU, Mana::C, 1);
    assert!(cast_option(&mut g, card).is_some(), "{{G}}{{2}} and their {{3}} pay {{5}}{{G}}");
    assert!(cast_option(&mut g, plain).is_none(), "no assist, no help");
}

/// CR 702.132a and the ruling: only generic mana - another player's green mana can't pay the {G}, so with no green mana of
/// its caster's the spell can't be cast.
#[test]
fn assist_pays_no_colored_mana() {
    let mut g = game();
    let card = put(&mut g, &BINOX, YOU, Zone::hand(YOU));
    add(&mut g, YOU, Mana::C, 5);
    add(&mut g, THEM, Mana::G, 6);
    assert!(cast_option(&mut g, card).is_none());
}

/// CR 702.132a: the chosen player pays part of the generic mana from their pool - announced, no more than the generic mana
/// - and the caster the rest.
#[test]
fn the_chosen_player_pays_part_of_the_generic_mana() {
    let mut g = game();
    let card = put(&mut g, &BINOX, YOU, Zone::hand(YOU));
    add(&mut g, YOU, Mana::G, 1);
    add(&mut g, YOU, Mana::C, 3);
    add(&mut g, THEM, Mana::C, 7);
    let option = cast_option(&mut g, card).expect("the cast");
    answer(&mut g, Answer::Act(Action::of(&option)));
    // The caster may choose another player (CR 702.132a).
    let r = next(&mut g);
    assert_eq!(r.player, YOU);
    assert!(matches!(r.kind, RequestKind::May { purpose: Purpose::Assist, .. }), "{:?}", r.kind);
    g.answer(r.id, r.player, Answer::Yes).expect("legal");
    // Neither has a mana ability, so the chosen player is asked straight away how much of the {5} they pay: at most 5,
    // though their pool holds 7.
    let r = next(&mut g);
    assert_eq!(r.player, THEM);
    let RequestKind::PayCosts { announcing: true, most, assist: AssistStep::HelperAnnounces, .. } = r.kind else { panic!("{:?}", r.kind) };
    assert_eq!(most, 5);
    assert!(g.answer(r.id, r.player, Answer::Announce(6)).is_err(), "more than the generic mana is refused");
    g.answer(r.id, r.player, Answer::Announce(2)).expect("two of it");
    pay_rest(&mut g);
    assert_eq!(g.state.player(THEM).pool.total(), 5, "they paid exactly {{2}}");
    assert_eq!(g.state.player(YOU).pool.total(), 0, "you paid {{3}}{{G}}");
    let spell = g.state.stack.last().expect("the spell").object;
    assert_eq!(g.state.obj(spell).controller, YOU);
    assert_eq!(g.state.obj(spell).spent.total(), 6, "all the mana spent to cast it, theirs included");
}

/// CR 702.132a: the caster may choose no one, and pays it all.
#[test]
fn the_caster_may_choose_no_one() {
    let mut g = game();
    let card = put(&mut g, &BINOX, YOU, Zone::hand(YOU));
    add(&mut g, YOU, Mana::G, 1);
    add(&mut g, YOU, Mana::C, 5);
    add(&mut g, THEM, Mana::C, 5);
    let option = cast_option(&mut g, card).expect("the cast");
    answer(&mut g, Answer::Act(Action::of(&option)));
    answer(&mut g, Answer::No);
    pay_rest(&mut g);
    assert_eq!(g.state.player(THEM).pool.total(), 5);
    assert_eq!(g.state.player(YOU).pool.total(), 0);
}

/// CR 702.132a, 601.2g: the chosen player activates mana abilities first, then the caster; then the chosen player pays, and
/// the caster pays the rest with no mana ability offered any more.
#[test]
fn the_chosen_player_activates_mana_abilities_first_and_none_are_offered_once_paying_begins() {
    let mut g = game();
    let card = put(&mut g, &BINOX, YOU, Zone::hand(YOU));
    let theirs = put(&mut g, &FOREST, THEM, Zone::BATTLEFIELD);
    let yours = put(&mut g, &FOREST, YOU, Zone::BATTLEFIELD);
    add(&mut g, YOU, Mana::C, 4);
    let option = cast_option(&mut g, card).expect("the cast");
    answer(&mut g, Answer::Act(Action::of(&option)));
    answer(&mut g, Answer::Yes);
    // Their window: their Forest.
    let r = next(&mut g);
    assert_eq!(r.player, THEM);
    let RequestKind::PayCosts { options, assist: AssistStep::HelperActivates, total, .. } = &r.kind else { panic!("{:?}", r.kind) };
    assert_eq!(*total, generic(5), "they are shown the generic mana they may pay");
    let tap = options.iter().find(|o| o.kind == ActionKind::ActivateAbility && o.source == Some(theirs)).expect("their Forest").clone();
    assert!(!options.iter().any(|o| o.source == Some(yours)));
    assert!(!options.iter().any(|o| o.kind == ActionKind::CancelCasting), "the spell is not theirs to take back");
    g.answer(r.id, r.player, Answer::Act(Action::of(&tap))).expect("legal");
    // Asked again until they say they are done (none left to activate: the window closes by itself).
    // The caster's window: their Forest.
    let r = next(&mut g);
    assert_eq!(r.player, YOU);
    let RequestKind::PayCosts { options, assist: AssistStep::CasterActivates, .. } = &r.kind else { panic!("{:?}", r.kind) };
    let tap = options.iter().find(|o| o.kind == ActionKind::ActivateAbility && o.source == Some(yours)).expect("your Forest").clone();
    g.answer(r.id, r.player, Answer::Act(Action::of(&tap))).expect("legal");
    // They announce: their {G} may pay one generic.
    let r = next(&mut g);
    assert_eq!(r.player, THEM);
    let RequestKind::PayCosts { most, assist: AssistStep::HelperAnnounces, .. } = r.kind else { panic!("{:?}", r.kind) };
    assert_eq!(most, 1);
    g.answer(r.id, r.player, Answer::Announce(1)).expect("legal");
    // Their {G} pays it without a question, and the caster's {G}{4} pays the rest.
    pay_rest(&mut g);
    assert_eq!(g.state.player(THEM).pool.total(), 0);
    assert_eq!(g.state.player(YOU).pool.total(), 0);
    assert!(g.state.stack.last().is_some_and(|i| g.state.obj(i.object).controller == YOU));
}

/// CR 601.2g: once paying with assist begins, the caster is offered no mana ability - one short of what is owed, they can
/// only take the cast back.
#[test]
fn the_caster_pays_with_no_mana_ability_once_paying_begins() {
    let mut g = game();
    let card = put(&mut g, &BINOX, YOU, Zone::hand(YOU));
    let yours = put(&mut g, &FOREST, YOU, Zone::BATTLEFIELD);
    add(&mut g, YOU, Mana::C, 3);
    add(&mut g, THEM, Mana::C, 2);
    let option = cast_option(&mut g, card).expect("the cast");
    answer(&mut g, Answer::Act(Action::of(&option)));
    answer(&mut g, Answer::Yes);
    // The caster leaves their Forest untapped.
    act(&mut g, ActionKind::ConfirmCasting);
    answer(&mut g, Answer::Announce(2));
    let r = next(&mut g);
    assert_eq!(r.player, YOU);
    let RequestKind::PayCosts { options, assist: AssistStep::CasterPays, .. } = &r.kind else { panic!("{:?}", r.kind) };
    assert!(!options.iter().any(|o| o.source == Some(yours)), "no mana ability once paying has begun");
    assert!(options.iter().any(|o| o.kind == ActionKind::CancelCasting));
}

/// CR 702.132a: "another player" - the offer counts on what a player other than the caster could pay, never the caster's own
/// mana a second time: {G}{4} of yours and nothing of theirs is one short of {5}{G}.
#[test]
fn the_offer_never_counts_the_casters_own_mana_as_help() {
    let mut g = game();
    let card = put(&mut g, &BINOX, YOU, Zone::hand(YOU));
    add(&mut g, YOU, Mana::G, 1);
    add(&mut g, YOU, Mana::C, 4);
    assert!(cast_option(&mut g, card).is_none());
}

/// CR 106.6: the assisting player's mana is spent on the spell, so what that mana does to the spell it is spent on, it does:
/// mana that gives a creature spell haste (Arena of Glory's) gives the assisted creature spell haste.
#[test]
fn what_the_assisting_players_mana_does_to_a_spell_it_does_to_this_one() {
    let mut g = game();
    let card = put(&mut g, &BINOX, YOU, Zone::hand(YOU));
    add(&mut g, YOU, Mana::G, 1);
    add(&mut g, YOU, Mana::C, 4);
    let source = g.state.zone(Zone::library(THEM))[0];
    g.state.add_mana(THEM, Mana::R, 1, source, Some(ManaRestriction::GainsHaste), ManaLasts::Step);
    let option = cast_option(&mut g, card).expect("the cast");
    answer(&mut g, Answer::Act(Action::of(&option)));
    answer(&mut g, Answer::Yes);
    answer(&mut g, Answer::Announce(1));
    pay_rest(&mut g);
    let spell = g.state.stack.last().expect("the spell").object;
    assert_eq!(g.state.player(THEM).pool.total(), 0, "their {{R}} paid {{1}}");
    assert!(g.state.obj(spell).marks.gains_haste, "the mana they spent gives it haste");
}

/// CR 702.132a: the assisting player pays what they announced; the spell is not theirs to take back while they pay it.
#[test]
fn the_assisting_player_cannot_take_back_the_spell_while_paying() {
    let mut g = game();
    let card = put(&mut g, &BINOX, YOU, Zone::hand(YOU));
    add(&mut g, YOU, Mana::G, 1);
    add(&mut g, YOU, Mana::C, 4);
    add(&mut g, THEM, Mana::R, 1);
    add(&mut g, THEM, Mana::U, 1);
    let option = cast_option(&mut g, card).expect("the cast");
    answer(&mut g, Answer::Act(Action::of(&option)));
    answer(&mut g, Answer::Yes);
    answer(&mut g, Answer::Announce(1));
    // Which of their two mana pays the {1} is theirs to choose: a question, with no way to cancel.
    let r = next(&mut g);
    assert_eq!(r.player, THEM);
    let RequestKind::PayCosts { options, assist: AssistStep::HelperPays, .. } = &r.kind else { panic!("{:?}", r.kind) };
    assert!(!options.iter().any(|o| o.kind == ActionKind::CancelCasting), "no CancelCasting offered");
    assert!(g.answer(r.id, r.player, Answer::Cancel).is_err(), "and none taken");
    pay_rest(&mut g);
    assert_eq!(g.state.player(THEM).pool.total(), 1);
    assert!(g.state.stack.last().is_some_and(|i| g.state.obj(i.object).controller == YOU));
}
