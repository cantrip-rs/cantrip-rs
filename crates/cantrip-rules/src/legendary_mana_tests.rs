//! Mana spendable only on a legendary spell (CR 106.6), over test cards, since no card in the engine prints it alone yet:
//! it casts a legendary spell, is no help to any other, and does nothing to the spell it pays for. Delighted Halfling's
//! is the same limit, with an effect, and is mana of its own kind.

use crate::{Game, Setup};
use cantrip_api::*;
use cantrip_state::{Frame, FrameKind};

const YOU: PlayerId = PlayerId(0);

static FILLER: CardDef = CardDef { name: "Legendary Mana Test Filler", has_cost: false, types: TypeSet::of(&[CardType::Land]), ..CardDef::BLANK };

/// A legendary 2/2 for {G}.
static LEGEND: CardDef = CardDef {
    name: "Legendary Mana Test Legend",
    cost: mana("{G}"),
    types: TypeSet::of(&[CardType::Creature]),
    supertypes: Supertypes { basic: false, legendary: true, snow: false },
    power: Some(2),
    toughness: Some(2),
    ..CardDef::BLANK
};

/// The same creature, not legendary.
static COMMONER: CardDef = CardDef { name: "Legendary Mana Test Commoner", supertypes: Supertypes { basic: false, legendary: false, snow: false }, ..LEGEND };

/// A legendary creature for {G}{G}.
static TWO_GREEN: CardDef = CardDef { name: "Legendary Mana Test Two Green", cost: mana("{G}{G}"), ..LEGEND };

/// A legendary creature for {R}{G}.
static GRUUL: CardDef = CardDef { name: "Legendary Mana Test Gruul", cost: mana("{R}{G}"), ..LEGEND };

/// A legendary creature for {G/R}.
static HYBRID: CardDef = CardDef { name: "Legendary Mana Test Hybrid", cost: mana("{G/R}"), ..LEGEND };

/// A legendary creature for {1}.
static GENERIC: CardDef = CardDef { name: "Legendary Mana Test Generic", cost: mana("{1}"), ..LEGEND };

/// A legendary creature for {G} with convoke, whose payment offers something besides mana (CR 702.51a).
static CONVOKER: CardDef = CardDef { name: "Legendary Mana Test Convoker", pay_with: PayWith { convoke: true, ..PayWith::NONE }, ..LEGEND };

fn nothing(_: &mut dyn cantrip_api::Game, _: &StackItem) -> Result<(), Suspend> {
    Ok(())
}

/// A creature with "{G}: Nothing happens."
static TINKERER: CardDef = CardDef { name: "Legendary Mana Test Tinkerer", supertypes: Supertypes { basic: false, legendary: false, snow: false }, abilities: &[ability(cost("{G}"), nothing, "{G}: Nothing happens.")], ..LEGEND };

fn game() -> Game {
    let table: &'static CardTable = Box::leak(Box::new(CardTable::new([&FILLER, &LEGEND, &COMMONER, &TWO_GREEN, &GRUUL, &HYBRID, &GENERIC, &CONVOKER, &TINKERER])));
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

fn in_hand(g: &mut Game, def: &'static CardDef) -> ObjectId {
    let zone = Zone::hand(YOU);
    let id = g.state.push_obj(Obj::new(g.cards.id_of(def), YOU, zone));
    g.state.zone_push(zone, id);
    id
}

fn next(g: &mut Game) -> Request {
    g.run();
    g.pending().cloned().expect("a question")
}

fn cast_option(g: &mut Game, card: ObjectId) -> Option<ActionOption> {
    match next(g).kind {
        RequestKind::Priority { options } => options.into_iter().find(|o| o.kind == ActionKind::CastSpell && o.source == Some(card)),
        other => panic!("expected priority, got {other:?}"),
    }
}

/// Casts the card with the restricted {G} in the pool, and hands back the spell.
fn cast_with(restriction: ManaRestriction) -> (Game, ObjectId) {
    let mut g = game();
    let card = in_hand(&mut g, &LEGEND);
    let source = g.state.zone(Zone::library(YOU))[0];
    g.state.add_mana(YOU, Mana::G, 1, source, Some(restriction), ManaLasts::Step, false);
    let option = cast_option(&mut g, card).expect("the mana casts a legendary spell");
    let r = next(&mut g);
    g.answer(r.id, r.player, Answer::Act(Action::of(&option))).expect("a legal answer");
    loop {
        let r = next(&mut g);
        let RequestKind::PayCosts { options, .. } = &r.kind else { break };
        let pay = options.iter().find(|o| matches!(o.kind, ActionKind::ConfirmCasting | ActionKind::AssignMana)).expect("the pool pays").clone();
        g.answer(r.id, r.player, Answer::Act(Action::of(&pay))).expect("a legal answer");
    }
    let spell = g.state.stack.last().expect("the spell").object;
    assert_eq!(g.state.player(YOU).pool.total() + g.state.restricted_of(YOU).map(|(_, pool)| pool.total()).sum::<u32>(), 0, "the mana is spent");
    (g, spell)
}

/// CR 106.6: "spend this mana only to cast a legendary spell", and that is all it says: the spell can be countered.
#[test]
fn it_casts_a_legendary_spell_and_does_nothing_to_it() {
    let (g, spell) = cast_with(ManaRestriction::LegendarySpells);
    assert!(!g.state.obj(spell).marks.cant_be_countered && !g.state.obj(spell).marks.gains_haste);
}

/// The positive control: the same payment with Delighted Halfling's mana is what makes a spell uncounterable.
#[test]
fn the_mana_that_also_says_it_cant_be_countered_does_that() {
    let (g, spell) = cast_with(ManaRestriction::LegendaryUncounterable);
    assert!(g.state.obj(spell).marks.cant_be_countered);
}

/// CR 106.6: it is no help to a spell that is not legendary, so with nothing else that spell is not offered.
#[test]
fn it_does_not_cast_a_spell_that_is_not_legendary() {
    let mut g = game();
    let commoner = in_hand(&mut g, &COMMONER);
    let source = g.state.zone(Zone::library(YOU))[0];
    g.state.add_mana(YOU, Mana::G, 1, source, Some(ManaRestriction::LegendarySpells), ManaLasts::Step, false);
    assert!(cast_option(&mut g, commoner).is_none());
}

const ALONE: ManaRestriction = ManaRestriction::LegendarySpells;
const UNCOUNTERABLE: ManaRestriction = ManaRestriction::LegendaryUncounterable;

/// `floating` in the pool, in that order.
fn float(g: &mut Game, floating: &[(Mana, Option<ManaRestriction>)]) {
    let source = g.state.zone(Zone::library(YOU))[0];
    for &(kind, restriction) in floating {
        g.state.add_mana(YOU, kind, 1, source, restriction, ManaLasts::Step, false);
    }
}

/// The game at the payment for a `def` cast with `floating` in the pool.
fn paying_for(def: &'static CardDef, floating: &[(Mana, Option<ManaRestriction>)]) -> Game {
    let mut g = game();
    let card = in_hand(&mut g, def);
    float(&mut g, floating);
    let option = cast_option(&mut g, card).expect("the mana casts it");
    let r = next(&mut g);
    g.answer(r.id, r.player, Answer::Act(Action::of(&option))).expect("a legal answer");
    g
}

/// What the payment asked now offers of the pool, and whether it offers confirming the pool's one way.
fn offered(g: &mut Game) -> (Vec<(Mana, Option<ManaRestriction>)>, bool) {
    let RequestKind::PayCosts { options, .. } = next(g).kind else { panic!("expected a payment") };
    (options.iter().filter(|o| o.kind == ActionKind::AssignMana).map(|o| (o.mana.expect("its mana"), o.restriction)).collect(), options.iter().any(|o| o.kind == ActionKind::ConfirmCasting))
}

/// Answers the payment with the pool's mana of this kind.
fn spend(g: &mut Game, kind: Mana, restriction: ManaRestriction) {
    let r = next(g);
    let RequestKind::PayCosts { options, .. } = &r.kind else { panic!("expected a payment, got {:?}", r.kind) };
    let pay = options.iter().find(|o| o.kind == ActionKind::AssignMana && o.mana == Some(kind) && o.restriction == Some(restriction)).expect("that mana is offered").clone();
    g.answer(r.id, r.player, Answer::Act(Action::of(&pay))).expect("a legal answer");
    next(g);
}

fn uncounterable(g: &Game) -> bool {
    g.state.obj(g.state.stack.last().expect("the spell").object).marks.cant_be_countered
}

/// The restricted mana still in the pool.
fn left(g: &Game) -> Vec<(ManaRestriction, ManaPool)> {
    g.state.restricted_of(YOU).filter(|(_, pool)| pool.total() > 0).collect()
}

/// CR 106.6, 601.2h: {G} of two restricted kinds, paying {G}. Which pays is the player's to choose, whichever was made first,
/// and it decides whether the spell can't be countered and which mana stays floating.
#[test]
fn two_restricted_kinds_of_one_type_are_the_players_choice() {
    for made in [[ALONE, UNCOUNTERABLE], [UNCOUNTERABLE, ALONE]] {
        for (spent, stays) in [(ALONE, UNCOUNTERABLE), (UNCOUNTERABLE, ALONE)] {
            let mut g = paying_for(&LEGEND, &made.map(|r| (Mana::G, Some(r))));
            let (assign, confirm) = offered(&mut g);
            assert!(assign.len() == 2 && assign.contains(&(Mana::G, Some(ALONE))) && assign.contains(&(Mana::G, Some(UNCOUNTERABLE))), "{assign:?}");
            assert!(!confirm, "no way of paying is the pool's alone");
            spend(&mut g, Mana::G, spent);
            assert_eq!(uncounterable(&g), spent == UNCOUNTERABLE, "{spent:?} paid, {made:?} made");
            let left = left(&g);
            assert!(left.len() == 1 && left[0].0 == stays && left[0].1.get(Mana::G) == 1, "{left:?}");
        }
    }
}

/// Every kind of the type is spent, so there is nothing to choose: nothing is asked, and the spell gets what any of it does.
#[test]
fn spending_every_kind_of_a_type_asks_nothing() {
    let mut g = paying_for(&TWO_GREEN, &[(Mana::G, Some(ALONE)), (Mana::G, Some(UNCOUNTERABLE))]);
    let r = next(&mut g);
    assert!(matches!(r.kind, RequestKind::Priority { .. }), "paid without a question: {:?}", r.kind);
    assert!(uncounterable(&g));
}

/// A cost of several types: {R}{G}, the {R} ordinary and all of it spent, the {G} of two kinds.
#[test]
fn a_cost_of_several_types_asks_about_the_type_with_kinds() {
    let mut g = paying_for(&GRUUL, &[(Mana::R, None), (Mana::G, Some(ALONE)), (Mana::G, Some(UNCOUNTERABLE))]);
    let (assign, confirm) = offered(&mut g);
    assert!(!confirm && assign.contains(&(Mana::G, Some(ALONE))) && assign.contains(&(Mana::G, Some(UNCOUNTERABLE))), "{assign:?}");
    spend(&mut g, Mana::G, UNCOUNTERABLE);
    // The {R} is all of its type, so it is paid with nothing more asked.
    assert!(matches!(next(&mut g).kind, RequestKind::Priority { .. }));
    assert!(uncounterable(&g));
    assert_eq!(g.state.player(YOU).pool.total(), 0);
    let left = left(&g);
    assert!(left.len() == 1 && left[0].0 == ALONE, "{left:?}");
}

/// A hybrid symbol one color of which the pool holds (CR 107.4e), and a generic one (CR 107.4b), are the same choice.
#[test]
fn hybrid_and_generic_symbols_are_the_same_choice() {
    for def in [&HYBRID, &GENERIC] {
        let mut g = paying_for(def, &[(Mana::G, Some(ALONE)), (Mana::G, Some(UNCOUNTERABLE))]);
        let (assign, confirm) = offered(&mut g);
        assert!(assign.len() == 2 && !confirm, "{}: {assign:?}", def.name);
        spend(&mut g, Mana::G, UNCOUNTERABLE);
        assert!(uncounterable(&g), "{}", def.name);
    }
}

/// CR 601.2h: with a creature to convoke as well, the pool is still offered kind by kind, not as one way to confirm.
#[test]
fn something_else_that_could_pay_leaves_the_choice_of_kinds() {
    let mut g = game();
    let card = in_hand(&mut g, &CONVOKER);
    let creature = g.state.push_obj(Obj::new(g.cards.id_of(&COMMONER), YOU, Zone::BATTLEFIELD));
    g.state.zone_push(Zone::BATTLEFIELD, creature);
    float(&mut g, &[(Mana::G, Some(ALONE)), (Mana::G, Some(UNCOUNTERABLE))]);
    let option = cast_option(&mut g, card).expect("the mana casts it");
    let r = next(&mut g);
    g.answer(r.id, r.player, Answer::Act(Action::of(&option))).expect("a legal answer");
    let (assign, confirm) = offered(&mut g);
    assert!(!confirm && assign.contains(&(Mana::G, Some(ALONE))) && assign.contains(&(Mana::G, Some(UNCOUNTERABLE))), "{assign:?}");
    spend(&mut g, Mana::G, UNCOUNTERABLE);
    assert!(uncounterable(&g));
}

/// CR 106.6, 602.2b: an ability's cost is the same choice: {G} only for creature abilities, or {G} that may go on anything and
/// hastes a creature spell. Which pays decides which is left for that spell.
#[test]
fn an_abilitys_cost_is_the_same_choice() {
    for (spent, stays) in [(ManaRestriction::CREATURE_ABILITIES, ManaRestriction::GainsHaste), (ManaRestriction::GainsHaste, ManaRestriction::CREATURE_ABILITIES)] {
        let mut g = game();
        let tinkerer = g.state.push_obj(Obj::new(g.cards.id_of(&TINKERER), YOU, Zone::BATTLEFIELD));
        g.state.zone_push(Zone::BATTLEFIELD, tinkerer);
        float(&mut g, &[(Mana::G, Some(ManaRestriction::CREATURE_ABILITIES)), (Mana::G, Some(ManaRestriction::GainsHaste))]);
        let RequestKind::Priority { options } = next(&mut g).kind else { panic!("expected priority") };
        let activate = options.into_iter().find(|o| o.kind == ActionKind::ActivateAbility && o.source == Some(tinkerer)).expect("the ability");
        let r = next(&mut g);
        g.answer(r.id, r.player, Answer::Act(Action::of(&activate))).expect("a legal answer");
        let (assign, confirm) = offered(&mut g);
        assert!(assign.len() == 2 && !confirm, "{assign:?}");
        spend(&mut g, Mana::G, spent);
        let left = left(&g);
        assert!(left.len() == 1 && left[0].0 == stays, "{left:?}");
    }
}
