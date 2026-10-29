//! "X can't be 0" (CR 107.3a, 601.2b, 602.2b), over test cards: the announcement of an activated ability's X and of a
//! spell's X starts at 1 and refuses 0; neither is offered unless it could be paid with X as 1; and a spell is not cast
//! for a cost where X could only be 0 (CR 107.3b; Mind Grind's ruling), nor an ability activated for an alternative cost with
//! no {X} (CR 118.9 by way of 602.2b).

use crate::actions::priority_options;
use crate::{Cx, Game, Setup};
use cantrip_api::*;
use cantrip_state::{Frame, FrameKind};

const YOU: PlayerId = PlayerId(0);

fn nothing(_: &mut dyn cantrip_api::Game, _: &StackItem) -> Result<(), Suspend> {
    Ok(())
}

static FILLER: CardDef = CardDef { name: "X Test Filler", has_cost: false, types: TypeSet::of(&[CardType::Land]), ..CardDef::BLANK };

/// "{X}{G}: ... X can't be 0." (Lair of the Hydra's cost.)
static LAIR: CardDef = CardDef {
    name: "X Test Lair",
    has_cost: false,
    types: TypeSet::of(&[CardType::Land]),
    abilities: &[ability(of_mana(mana("{X}{G}")), nothing, "{X}{G}: Nothing. X can't be 0.").x_cant_be_zero()],
    ..CardDef::BLANK
};

/// "{X}{G}: ..." with no word on X: X may be 0.
static DEN: CardDef = CardDef {
    name: "X Test Den",
    has_cost: false,
    types: TypeSet::of(&[CardType::Land]),
    abilities: &[ability(of_mana(mana("{X}{G}")), nothing, "{X}{G}: Nothing.")],
    ..CardDef::BLANK
};

/// A {X}{U} sorcery: "X can't be 0." It may also be cast for {U}, a cost with no {X}.
static GRIND: CardDef = CardDef {
    name: "X Test Grind",
    cost: mana("{X}{U}"),
    types: TypeSet::of(&[CardType::Sorcery]),
    spell: spell(&[], nothing),
    ways: &[way(of_mana(mana("{U}")), ZoneKind::Hand, "You may pay {U} rather than pay this spell's mana cost.")],
    x_cant_be_zero: true,
    ..CardDef::BLANK
};

/// A {X}{U} sorcery that may be cast for {X}{R} instead, a way of its own whose {X} is announced (CR 118.9, 107.3a): with no
/// word on X, and with "X can't be 0".
static SPLASH: CardDef = CardDef {
    name: "X Test Splash",
    cost: mana("{X}{U}"),
    types: TypeSet::of(&[CardType::Sorcery]),
    spell: spell(&[], nothing),
    ways: &[way(of_mana(mana("{X}{R}")), ZoneKind::Hand, "You may pay {X}{R} rather than pay this spell's mana cost.")],
    ..CardDef::BLANK
};

static SURGE: CardDef = CardDef { name: "X Test Surge", x_cant_be_zero: true, ..SPLASH };

/// The same sorcery with no word on X.
static SIFT: CardDef = CardDef {
    name: "X Test Sift",
    cost: mana("{X}{U}"),
    types: TypeSet::of(&[CardType::Sorcery]),
    spell: spell(&[], nothing),
    ways: &[way(of_mana(mana("{U}")), ZoneKind::Hand, "You may pay {U} rather than pay this spell's mana cost.")],
    ..CardDef::BLANK
};

/// "You may pay {0} rather than pay the activation cost of abilities you activate" (as Kili's {0} equip, CR 118.9, 602.2b).
fn for_nothing(_: &dyn Query, asker: &Asker, activator: PlayerId, _: ObjectId, _: &ActivatedDef) -> Option<ManaCost> {
    (activator == asker.controller).then_some(ManaCost::ZERO)
}

static SMITH: CardDef = CardDef {
    name: "X Test Smith",
    types: TypeSet::of(&[CardType::Artifact]),
    ability_alternatives: &[for_nothing],
    ..CardDef::BLANK
};

fn a_creature(q: &dyn Query, _: &Asker, t: Target) -> bool {
    matches!(t, Target::Object(id) if q.is_on_battlefield(id) && q.chars(id).is(CardType::Creature))
}

/// "{X}: Nothing happens to X target creatures. X can't be 0."
static CHAINER: CardDef = CardDef {
    name: "X Test Chainer",
    types: TypeSet::of(&[CardType::Artifact]),
    abilities: &[ability(of_mana(mana("{X}")), nothing, "{X}: Nothing happens to X target creatures. X can't be 0.")
        .targets(&[target(a_creature).counted()])
        .x_cant_be_zero()],
    ..CardDef::BLANK
};

/// The same with no word on X: X may be 0, with no target.
static BINDER: CardDef = CardDef {
    name: "X Test Binder",
    types: TypeSet::of(&[CardType::Artifact]),
    abilities: &[ability(of_mana(mana("{X}")), nothing, "{X}: Nothing happens to X target creatures.").targets(&[target(a_creature).counted()])],
    ..CardDef::BLANK
};

static BEAR: CardDef = CardDef { name: "X Test Bear", types: TypeSet::of(&[CardType::Creature]), power: Some(2), toughness: Some(2), ..CardDef::BLANK };

/// The card named `name` in the resolving effect's controller's hand.
fn in_hand(g: &dyn cantrip_api::Game, item: &StackItem, name: &str) -> ObjectId {
    g.state().zone(Zone::hand(item.controller)).iter().copied().find(|&c| g.chars(c).name == name).expect("in hand")
}

/// "Cast X Test Grind without paying its mana cost" (CR 608.2g, 118.9).
fn cast_grind_free(g: &mut dyn cantrip_api::Game, item: &StackItem) -> Result<(), Suspend> {
    let grind = in_hand(g, item, "X Test Grind");
    let cast = g.cast_during_resolution(grind, Some(ManaCost::ZERO))?;
    assert!(!cast, "an X that can't be 0 has nothing to be when cast without paying its mana cost");
    Ok(())
}

/// "Cast X Test Grind" paying its costs: its mana cost or a way of its own.
fn cast_grind(g: &mut dyn cantrip_api::Game, item: &StackItem) -> Result<(), Suspend> {
    let grind = in_hand(g, item, "X Test Grind");
    g.cast_during_resolution(grind, None)?;
    Ok(())
}

/// "Cast X Test Sift" paying its costs.
fn cast_sift(g: &mut dyn cantrip_api::Game, item: &StackItem) -> Result<(), Suspend> {
    let sift = in_hand(g, item, "X Test Sift");
    g.cast_during_resolution(sift, None)?;
    Ok(())
}

static FREE_CASTER: CardDef = CardDef { name: "X Test Free Caster", types: TypeSet::of(&[CardType::Sorcery]), spell: spell(&[], cast_grind_free), ..CardDef::BLANK };
static GRIND_CASTER: CardDef = CardDef { name: "X Test Grind Caster", types: TypeSet::of(&[CardType::Sorcery]), spell: spell(&[], cast_grind), ..CardDef::BLANK };
static SIFT_CASTER: CardDef = CardDef { name: "X Test Sift Caster", types: TypeSet::of(&[CardType::Sorcery]), spell: spell(&[], cast_sift), ..CardDef::BLANK };

/// Your precombat main phase with priority, nothing anywhere.
fn game() -> Game {
    let table: &'static CardTable = Box::leak(Box::new(CardTable::new([
        &FILLER, &LAIR, &DEN, &GRIND, &SIFT, &SPLASH, &SURGE, &SMITH, &CHAINER, &BINDER, &BEAR, &FREE_CASTER, &GRIND_CASTER, &SIFT_CASTER,
    ])));
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
    g.state.update_player(YOU, |p| p.turn_began = 1);
    g
}

fn put(g: &mut Game, def: &'static CardDef, zone: Zone) -> ObjectId {
    let id = g.state.push_obj(Obj::new(g.cards.id_of(def), YOU, zone));
    g.state.zone_push(zone, id);
    id
}

fn cx(game: &mut Game) -> Cx<'_> {
    Cx { g: game, frame: usize::MAX, cursor: 0, mana_source: None, entering: None, lineage: Vec::new(), asked_at: None, doer: None, producing: None }
}

fn add_mana(g: &mut Game, pool: &[Mana]) {
    let source = g.state.battlefield().first().copied().unwrap_or(ObjectId(0));
    for &m in pool {
        g.state.add_mana(YOU, m, 1, source, None, ManaLasts::Step);
    }
}

fn next(g: &mut Game) -> Request {
    g.run();
    g.pending().cloned().expect("a question")
}

/// The ways `source` is offered at `YOU`'s priority: an ability's index, or a cast's way number.
fn offered(g: &mut Game, source: ObjectId) -> Vec<u8> {
    priority_options(&mut cx(g), YOU).iter().filter(|o| o.source == Some(source)).map(|o| o.index).collect()
}

/// Takes the first option `source` has, and gives back the announcement it asks.
fn announce(g: &mut Game, source: ObjectId) -> Request {
    let r = next(g);
    let RequestKind::Priority { options } = &r.kind else { panic!("expected priority, got {:?}", r.kind) };
    let option = options.iter().find(|o| o.source == Some(source) && o.index == 0).expect("it is offered").clone();
    g.answer(r.id, r.player, Answer::Act(Action::of(&option))).expect("a legal answer");
    let r = next(g);
    assert!(matches!(r.kind, RequestKind::PayCosts { announcing: true, .. }), "expected the announcement of X, got {:?}", r.kind);
    r
}

fn kinds(r: &Request) -> Vec<ActionKind> {
    let RequestKind::PayCosts { options, .. } = &r.kind else { panic!("expected a payment, got {:?}", r.kind) };
    options.iter().map(|o| o.kind).collect()
}

#[test]
fn an_ability_whose_x_cant_be_0_is_offered_only_where_x_as_1_could_be_paid() {
    let mut g = game();
    let lair = put(&mut g, &LAIR, Zone::BATTLEFIELD);
    let den = put(&mut g, &DEN, Zone::BATTLEFIELD);
    add_mana(&mut g, &[Mana::G]);
    assert!(offered(&mut g, lair).is_empty(), "{{G}} pays X as 0 only, which the Lair's X can't be");
    assert_eq!(offered(&mut g, den), vec![0], "the Den's X may be 0");
    add_mana(&mut g, &[Mana::C]);
    assert_eq!(offered(&mut g, lair), vec![0], "{{1}}{{G}} pays X as 1");
}

#[test]
fn an_ability_whose_x_cant_be_0_announces_from_1_and_refuses_0() {
    let mut g = game();
    let lair = put(&mut g, &LAIR, Zone::BATTLEFIELD);
    add_mana(&mut g, &[Mana::G, Mana::C, Mana::C]);
    let r = announce(&mut g, lair);
    let RequestKind::PayCosts { x, least, total, .. } = &r.kind else { unreachable!() };
    assert_eq!((*x, *least), (1, 1), "the announcement starts at 1");
    assert_eq!(total.generic, 1, "{{X}}{{G}} shown at X 1");
    assert!(!kinds(&r).contains(&ActionKind::DecreaseX), "X can't be lowered to 0");
    assert!(g.answer(r.id, r.player, Answer::Announce(0)).is_err(), "0 is refused (CR 107.3a)");
    g.answer(r.id, r.player, Answer::Announce(2)).expect("2 may be announced");
    let r = next(&mut g);
    if let RequestKind::PayCosts { options, .. } = &r.kind {
        let confirm = options.iter().find(|o| o.kind == ActionKind::ConfirmCasting).expect("the pool pays it").clone();
        g.answer(r.id, r.player, Answer::Act(Action::of(&confirm))).expect("a legal answer");
    }
    next(&mut g);
    assert_eq!(g.state.stack.last().map(|i| (i.object, i.x)), Some((lair, 2)));
    assert_eq!(g.state.player(YOU).pool.total(), 0, "{{2}}{{G}} paid");
}

#[test]
fn an_ability_whose_x_may_be_0_still_announces_from_0() {
    let mut g = game();
    let den = put(&mut g, &DEN, Zone::BATTLEFIELD);
    add_mana(&mut g, &[Mana::G]);
    let r = announce(&mut g, den);
    let RequestKind::PayCosts { x, least, .. } = &r.kind else { unreachable!() };
    assert_eq!((*x, *least), (0, 0));
    g.answer(r.id, r.player, Answer::Announce(0)).expect("0 may be announced");
}

#[test]
fn a_spell_whose_x_cant_be_0_is_cast_with_x_1_or_more_and_never_for_a_cost_without_x() {
    let mut g = game();
    let grind = put(&mut g, &GRIND, Zone::hand(YOU));
    let sift = put(&mut g, &SIFT, Zone::hand(YOU));
    add_mana(&mut g, &[Mana::U]);
    // {U} pays the mana cost with X as 0, which the Grind's X can't be, and its {U} way has no {X}, so X could only be 0
    // (CR 107.3b): neither is offered. The Sift is offered both ways.
    assert!(offered(&mut g, grind).is_empty());
    assert_eq!(offered(&mut g, sift), vec![0, 1]);
    add_mana(&mut g, &[Mana::C]);
    assert_eq!(offered(&mut g, grind), vec![0], "{{1}}{{U}} pays X as 1; the way with no {{X}} is still not offered");
    let r = announce(&mut g, grind);
    let RequestKind::PayCosts { x, least, total, .. } = &r.kind else { unreachable!() };
    assert_eq!((*x, *least, total.generic), (1, 1, 1));
    assert!(!kinds(&r).contains(&ActionKind::DecreaseX));
    assert!(g.answer(r.id, r.player, Answer::Announce(0)).is_err(), "0 is refused (CR 107.3a)");
    g.answer(r.id, r.player, Answer::Announce(1)).expect("1 may be announced");
}

/// Every option `source` has at `YOU`'s priority, with what pays for it instead of its cost.
fn options_of(g: &mut Game, source: ObjectId) -> Vec<(u8, Option<Grant>)> {
    priority_options(&mut cx(g), YOU).iter().filter(|o| o.source == Some(source)).map(|o| (o.index, o.paid_by)).collect()
}

#[test]
fn an_alternative_cost_with_no_x_is_no_way_to_activate_an_ability_whose_x_cant_be_0() {
    let mut g = game();
    put(&mut g, &SMITH, Zone::BATTLEFIELD);
    let lair = put(&mut g, &LAIR, Zone::BATTLEFIELD);
    let den = put(&mut g, &DEN, Zone::BATTLEFIELD);
    // Nothing in the pool: {0} pays the Den's ability (its X 0), not the Lair's, whose X can't be 0 (CR 107.3a, 118.9).
    assert!(options_of(&mut g, lair).is_empty());
    assert!(matches!(options_of(&mut g, den)[..], [(0, Some(_))]), "the Den is offered for {{0}}");
    // {1}{G}: the Lair for its own cost, with X 1, and still not for {0}.
    add_mana(&mut g, &[Mana::G, Mana::C]);
    assert_eq!(options_of(&mut g, lair), vec![(0, None)]);
}

#[test]
fn an_x_that_cant_be_0_counting_targets_is_not_offered_with_none_to_target() {
    let mut g = game();
    let chainer = put(&mut g, &CHAINER, Zone::BATTLEFIELD);
    let binder = put(&mut g, &BINDER, Zone::BATTLEFIELD);
    add_mana(&mut g, &[Mana::C]);
    // No creature: X could only be 0 (CR 601.2c), which the Chainer's can't be; the Binder's may.
    assert!(offered(&mut g, chainer).is_empty());
    assert_eq!(offered(&mut g, binder), vec![0]);
    put(&mut g, &BEAR, Zone::BATTLEFIELD);
    assert_eq!(offered(&mut g, chainer), vec![0]);
}

/// Casts `caster` from your hand for nothing and lets it resolve, giving back the next question after it.
fn resolve(g: &mut Game, caster: &'static CardDef) -> Request {
    let caster = put(g, caster, Zone::hand(YOU));
    let r = next(g);
    let RequestKind::Priority { options } = &r.kind else { panic!("expected priority, got {:?}", r.kind) };
    let option = options.iter().find(|o| o.source == Some(caster)).expect("the caster is offered").clone();
    g.answer(r.id, r.player, Answer::Act(Action::of(&option))).expect("a legal answer");
    loop {
        let r = next(g);
        // The spell on the stack is a new object (CR 400.7): known by its name.
        let on_stack = g.state.stack.iter().any(|i| g.chars(i.object).name.starts_with("X Test") && g.chars(i.object).name.ends_with("Caster"));
        match &r.kind {
            RequestKind::Priority { .. } if on_stack => g.answer(r.id, r.player, Answer::Pass).expect("passing"),
            _ => return r,
        }
    }
}

#[test]
fn a_spell_whose_x_cant_be_0_is_not_cast_without_paying_its_mana_cost() {
    let mut g = game();
    let grind = put(&mut g, &GRIND, Zone::hand(YOU));
    add_mana(&mut g, &[Mana::U, Mana::C, Mana::C]);
    resolve(&mut g, &FREE_CASTER);
    assert_eq!(g.state.obj(grind).zone, Zone::hand(YOU), "the Grind stays in the hand (Mind Grind's ruling)");
    assert!(g.state.stack.is_empty());
}

#[test]
fn a_cast_as_an_effect_resolves_offers_no_cost_with_no_x_for_an_x_that_cant_be_0() {
    // The Sift's caster chooses between its mana cost and its {U} way.
    let mut g = game();
    put(&mut g, &SIFT, Zone::hand(YOU));
    add_mana(&mut g, &[Mana::U, Mana::C]);
    let r = resolve(&mut g, &SIFT_CASTER);
    assert!(matches!(&r.kind, RequestKind::ChooseModes { modes, .. } if modes.len() == 2), "expected the choice of cost, got {:?}", r.kind);
    // The Grind's {U} way has no {X}: its mana cost is the only cost, and X is announced from 1.
    let mut g = game();
    put(&mut g, &GRIND, Zone::hand(YOU));
    add_mana(&mut g, &[Mana::U, Mana::C]);
    let r = resolve(&mut g, &GRIND_CASTER);
    assert!(matches!(r.kind, RequestKind::PayCosts { announcing: true, x: 1, least: 1, .. }), "expected X announced from 1, got {:?}", r.kind);
}

/// Casts `card` the way numbered `way` and gives back the question that follows.
fn cast_way(g: &mut Game, card: ObjectId, way: u8) -> Request {
    let r = next(g);
    let RequestKind::Priority { options } = &r.kind else { panic!("expected priority, got {:?}", r.kind) };
    let option = options.iter().find(|o| o.source == Some(card) && o.index == way).expect("that way is offered").clone();
    g.answer(r.id, r.player, Answer::Act(Action::of(&option))).expect("a legal answer");
    next(g)
}

#[test]
fn an_alternative_cost_of_the_cards_own_with_x_announces_x() {
    // No word on X: {R} pays its {X}{R} way with X 0, and X is announced, from 0.
    let mut g = game();
    let splash = put(&mut g, &SPLASH, Zone::hand(YOU));
    add_mana(&mut g, &[Mana::R, Mana::C, Mana::C]);
    assert_eq!(offered(&mut g, splash), vec![1]);
    let r = cast_way(&mut g, splash, 1);
    let RequestKind::PayCosts { announcing: true, x: 0, least: 0, total, .. } = &r.kind else { panic!("expected X announced, got {:?}", r.kind) };
    assert_eq!((total.x, total.symbols[Mana::R as usize]), (1, 1), "the way's {{X}}{{R}}");
    g.answer(r.id, r.player, Answer::Announce(2)).expect("2 may be announced");
    let r = next(&mut g);
    if let RequestKind::PayCosts { options, .. } = &r.kind {
        let confirm = options.iter().find(|o| o.kind == ActionKind::ConfirmCasting).expect("the pool pays it").clone();
        g.answer(r.id, r.player, Answer::Act(Action::of(&confirm))).expect("a legal answer");
        next(&mut g);
    }
    assert_eq!(g.state.stack.last().map(|i| i.x), Some(2), "X is 2 on the stack");
    assert_eq!(g.state.player(YOU).pool.total(), 0, "{{2}}{{R}} paid");

    // "X can't be 0": its {X}{R} way is a cost with an {X}, so it may be cast that way, with X from 1 - not for {R}.
    let mut g = game();
    let surge = put(&mut g, &SURGE, Zone::hand(YOU));
    add_mana(&mut g, &[Mana::R]);
    assert!(offered(&mut g, surge).is_empty(), "{{R}} pays X as 0 only");
    add_mana(&mut g, &[Mana::C]);
    assert_eq!(offered(&mut g, surge), vec![1]);
    let r = cast_way(&mut g, surge, 1);
    assert!(matches!(r.kind, RequestKind::PayCosts { announcing: true, x: 1, least: 1, .. }), "expected X announced from 1, got {:?}", r.kind);
    assert!(g.answer(r.id, r.player, Answer::Announce(0)).is_err());
}
