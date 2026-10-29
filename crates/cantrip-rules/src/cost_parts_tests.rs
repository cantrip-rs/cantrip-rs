//! Cost parts the engine once stopped at, over test cards: an activated ability's behold (CR 701.4a,
//! 602.2b), a mana ability whose cost returns a tapped permanent (CR 605.3a, 601.2g-h), and a cost paid as a spell
//! resolves that has parts other than mana and life (CR 118.3, 118.12).

use crate::actions::priority_options;
use crate::{Cx, Game, Setup};
use cantrip_api::*;
use cantrip_state::{Frame, FrameKind};

const YOU: PlayerId = PlayerId(0);

fn is_bear(q: &dyn Query, _: &Asker, id: ObjectId) -> bool {
    q.chars(id).name == "Cost Test Bear"
}

fn is_creature(q: &dyn Query, _: &Asker, id: ObjectId) -> bool {
    q.chars(id).is(CardType::Creature)
}

fn gain_three(g: &mut dyn cantrip_api::Game, item: &StackItem) -> Result<(), Suspend> {
    g.gain_life(item.controller, 3);
    Ok(())
}

static FILLER: CardDef = CardDef { name: "Cost Test Filler", has_cost: false, types: TypeSet::of(&[CardType::Land]), ..CardDef::BLANK };

static BEAR: CardDef = CardDef {
    name: "Cost Test Bear",
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(2),
    toughness: Some(2),
    ..CardDef::BLANK
};

/// "{T}: Add {G}."
static DORK: CardDef = CardDef {
    name: "Cost Test Dork",
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(1),
    toughness: Some(1),
    abilities: &[mana_ability(TAP, add(Mana::G), "{T}: Add {G}.")],
    ..CardDef::BLANK
};

/// "{1}, Behold a Bear: You gain 3 life."
static SEER: CardDef = CardDef {
    name: "Cost Test Seer",
    types: TypeSet::of(&[CardType::Artifact]),
    abilities: &[ability(cost("{1}").and(Cost { behold: Some((1, is_bear, "Bear")), ..Cost::NONE }), gain_three, "{1}, Behold a Bear: You gain 3 life.")],
    ..CardDef::BLANK
};

/// "Behold a creature: You gain 3 life." A creature itself, it may behold itself.
static GAZER: CardDef = CardDef {
    name: "Cost Test Gazer",
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(1),
    toughness: Some(1),
    abilities: &[ability(Cost { behold: Some((1, is_creature, "creature")), ..Cost::NONE }, gain_three, "Behold a creature: You gain 3 life.")],
    ..CardDef::BLANK
};

/// "{1}, Return a tapped creature you control to its owner's hand: Add {G}{G}{G}."
static RETURNER: CardDef = CardDef {
    name: "Cost Test Returner",
    types: TypeSet::of(&[CardType::Artifact]),
    abilities: &[mana_ability(cost("{1}").and(Cost::return_tapped_to_hand(1, is_creature)), add_n(Mana::G, 3), "{1}, Return a tapped creature you control to its owner's hand: Add {G}{G}{G}.")],
    ..CardDef::BLANK
};

/// "{T}, Return a tapped creature you control to its owner's hand: You gain 3 life."
static RECOILER: CardDef = CardDef {
    name: "Cost Test Recoiler",
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(1),
    toughness: Some(1),
    abilities: &[ability(TAP.and(Cost::return_tapped_to_hand(1, is_creature)), gain_three, "{T}, Return a tapped creature you control to its owner's hand: You gain 3 life.")],
    ..CardDef::BLANK
};

const PAY: Question = Question::new("Sacrifice", "pay {cost}?", "", "Pay", "Don't");

fn may_pay_with(g: &mut dyn cantrip_api::Game, item: &StackItem, cost: Cost) -> Result<(), Suspend> {
    if g.may_pay(item.controller, item.object, item.object, cost, &PAY)? {
        g.gain_life(item.controller, 5);
    }
    Ok(())
}

/// "You may pay {1} and sacrifice a creature. If you do, you gain 5 life."
fn sacrifice_one(g: &mut dyn cantrip_api::Game, item: &StackItem) -> Result<(), Suspend> {
    may_pay_with(g, item, cost("{1}").and(Cost::sacrifice(1, is_creature)))
}

/// "You may pay {1} and return a tapped creature you control to its owner's hand. If you do, you gain 5 life."
fn return_tapped(g: &mut dyn cantrip_api::Game, item: &StackItem) -> Result<(), Suspend> {
    may_pay_with(g, item, cost("{1}").and(Cost::return_tapped_to_hand(1, is_creature)))
}

/// "You may discard a card and pay 1 energy. If you do, you gain 5 life."
fn discard_and_energy(g: &mut dyn cantrip_api::Game, item: &StackItem) -> Result<(), Suspend> {
    may_pay_with(g, item, discard_cards(1).and(Cost { energy: 1, ..Cost::NONE }))
}

/// "You may pay {B/P}. If you do, you gain 5 life." - a Phyrexian symbol in a cost paid as a spell resolves, as Ice Cave's
/// payer meets one in a spell's mana cost: paid with {B} or 2 life (CR 107.4f, 118.13b).
fn phyrexian_cost(g: &mut dyn cantrip_api::Game, item: &StackItem) -> Result<(), Suspend> {
    may_pay_with(g, item, cost("{B/P}"))
}

/// "You may pay {2/G} 150 times and {2/U} 150 times. If you do, you gain 5 life." - more monocolored hybrid symbols than a
/// choice of modes holds, which no printed card asks: how each is paid is still announced (CR 107.4e, 118.13b).
fn many_twobrids(g: &mut dyn cantrip_api::Game, item: &StackItem) -> Result<(), Suspend> {
    let mut twobrid = [0; 5];
    twobrid[Color::Blue as usize] = 150;
    twobrid[Color::Green as usize] = 150;
    may_pay_with(g, item, Cost::mana(ManaCost { twobrid, ..ManaCost::ZERO }))
}

/// "You may sacrifice 300 creatures. If you do, you gain 5 life." - a count past a byte, which a cumulative upkeep reaches with
/// 300 age counters (CR 702.24a).
fn sacrifice_three_hundred(g: &mut dyn cantrip_api::Game, item: &StackItem) -> Result<(), Suspend> {
    may_pay_with(g, item, Cost::sacrifice(300, is_creature))
}

/// A loyalty cost, which no resolving spell asks for (CR 606.4): never paid.
fn loyalty_cost(g: &mut dyn cantrip_api::Game, item: &StackItem) -> Result<(), Suspend> {
    may_pay_with(g, item, Cost { loyalty: Some(-1), ..Cost::NONE })
}

fn high_life_cost(g: &mut dyn cantrip_api::Game, item: &StackItem) -> Result<(), Suspend> {
    may_pay_with(g, item, Cost { life: 65_536, ..Cost::NONE })
}

fn impossible_life_cost(g: &mut dyn cantrip_api::Game, item: &StackItem) -> Result<(), Suspend> {
    may_pay_with(g, item, Cost { life: i32::MAX as u32 + 1, ..Cost::NONE })
}

static OFFER: CardDef = CardDef { name: "Cost Test Offer", types: TypeSet::of(&[CardType::Sorcery]), spell: spell(&[], sacrifice_one), ..CardDef::BLANK };
static RECALL: CardDef = CardDef { name: "Cost Test Recall", types: TypeSet::of(&[CardType::Sorcery]), spell: spell(&[], return_tapped), ..CardDef::BLANK };
static TITHE: CardDef = CardDef { name: "Cost Test Tithe", types: TypeSet::of(&[CardType::Sorcery]), spell: spell(&[], discard_and_energy), ..CardDef::BLANK };
static TOLL: CardDef = CardDef { name: "Cost Test Toll", types: TypeSet::of(&[CardType::Sorcery]), spell: spell(&[], phyrexian_cost), ..CardDef::BLANK };
static HOARD: CardDef = CardDef { name: "Cost Test Hoard", types: TypeSet::of(&[CardType::Sorcery]), spell: spell(&[], many_twobrids), ..CardDef::BLANK };
static MASS_OFFER: CardDef = CardDef { name: "Cost Test Mass Offer", types: TypeSet::of(&[CardType::Sorcery]), spell: spell(&[], sacrifice_three_hundred), ..CardDef::BLANK };
/// "Put 300 -1/-1 counters on this creature: You gain 3 life." - a count past a byte (CR 702.24a, 118.3).
static WITHERER: CardDef = CardDef {
    name: "Cost Test Witherer",
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(1000),
    toughness: Some(1000),
    abilities: &[ability(put_counters(CounterKind::MinusOne, 300), gain_three, "Put 300 -1/-1 counters on this creature: You gain 3 life.")],
    ..CardDef::BLANK
};
static VOW: CardDef = CardDef { name: "Cost Test Vow", types: TypeSet::of(&[CardType::Sorcery]), spell: spell(&[], loyalty_cost), ..CardDef::BLANK };
static HIGH_LIFE: CardDef = CardDef { name: "Cost Test High Life", types: TypeSet::of(&[CardType::Sorcery]), spell: spell(&[], high_life_cost), ..CardDef::BLANK };
static IMPOSSIBLE_LIFE: CardDef = CardDef { name: "Cost Test Impossible Life", types: TypeSet::of(&[CardType::Sorcery]), spell: spell(&[], impossible_life_cost), ..CardDef::BLANK };

/// "Whenever a card is put into a hand or a library, you gain 2 life" (CR 603.10a: it looks back for a card all players can
/// see).
fn a_card_into_a_hand_or_library(q: &dyn Query, _: ObjectId, event: &Event) -> Option<Fired> {
    match event {
        Event::ZoneChange { old, to, .. } if (to.is(ZoneKind::Hand) || to.is(ZoneKind::Library)) && !q.obj(*old).token => Fired::PLAIN,
        _ => None,
    }
}

fn gain_two(g: &mut dyn cantrip_api::Game, item: &StackItem) -> Result<(), Suspend> {
    g.gain_life(item.controller, 2);
    Ok(())
}

static LIBRARIAN: CardDef = CardDef {
    name: "Cost Test Librarian",
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(1),
    toughness: Some(1),
    abilities: &[triggered(when(EventMask::ZONE_CHANGE, a_card_into_a_hand_or_library), gain_two, "Whenever a card is put into a hand or a library, you gain 2 life.")],
    ..CardDef::BLANK
};

/// An Arcane sorcery for {1}: "You gain 3 life."
static RITE: CardDef = CardDef {
    name: "Cost Test Rite",
    cost: ManaCost { generic: 1, ..ManaCost::ZERO },
    has_cost: true,
    types: TypeSet::of(&[CardType::Sorcery]),
    subtypes: SubtypeSet::of(&[Subtype::Arcane]),
    spell: spell(&[], gain_three),
    ..CardDef::BLANK
};

/// "You gain 3 life. Splice onto Arcane {1}."
static GRAFT: CardDef = CardDef {
    name: "Cost Test Graft",
    cost: ManaCost { generic: 1, ..ManaCost::ZERO },
    has_cost: true,
    types: TypeSet::of(&[CardType::Instant]),
    spell: spell(&[], gain_three),
    splice: Some(ManaCost { generic: 1, ..ManaCost::ZERO }),
    ..CardDef::BLANK
};

/// "{T}: Add {C}. Put each card in your hand on top of your library, and each Librarian you control into your graveyard,
/// at once": a mana ability whose effect moves a card the spell being cast revealed (CR 605.1a, 601.2g).
fn stow(g: &mut dyn cantrip_api::Game, asker: &Asker, _: Option<Color>) -> Result<(), Suspend> {
    g.add_mana(asker.controller, Mana::C, 1);
    let hand = g.state().zone(Zone::hand(asker.controller)).to_vec();
    let librarians: Vec<ObjectId> =
        g.state().battlefield().iter().copied().filter(|&id| g.chars(id).name == "Cost Test Librarian").collect();
    let moves: Vec<(ObjectId, Zone)> = hand
        .iter()
        .map(|&c| (c, Zone::library(asker.controller)))
        .chain(librarians.iter().map(|&l| (l, Zone::graveyard(asker.controller))))
        .collect();
    g.move_together(&moves, MoveReason::Other);
    Ok(())
}

static STOWER: CardDef = CardDef {
    name: "Cost Test Stower",
    types: TypeSet::of(&[CardType::Artifact]),
    abilities: &[mana_ability(TAP, add(Mana::C).by(stow), "{T}: Add {C}. Put each card in your hand on top of your library.")],
    ..CardDef::BLANK
};

/// Your precombat main phase with priority, nothing anywhere.
fn game() -> Game {
    let table: &'static CardTable = Box::leak(Box::new(CardTable::new([&FILLER, &BEAR, &DORK, &SEER, &GAZER, &RETURNER, &OFFER, &RECALL, &TITHE, &VOW, &RECOILER, &LIBRARIAN, &RITE, &GRAFT, &STOWER, &TOLL, &HOARD, &HIGH_LIFE, &IMPOSSIBLE_LIFE, &MASS_OFFER, &WITHERER])));
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
    // What is put on the battlefield below has been there since before the turn began: no creature is summoning sick.
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

fn add_mana(g: &mut Game, n: u32) {
    let source = g.state.battlefield().first().copied().unwrap_or(ObjectId(0));
    g.state.add_mana(YOU, Mana::C, n, source, None, ManaLasts::Step);
}

/// Runs to the next question and gives it back.
fn next(g: &mut Game) -> Request {
    g.run();
    g.pending().cloned().unwrap_or_else(|| panic!("no question: over {} outcome {:?} agenda {}", g.is_over(), g.state.outcome, g.state.agenda.len()))
}

fn answer(g: &mut Game, r: &Request, a: Answer) {
    g.answer(r.id, r.player, a).expect("a legal answer");
}

/// The option `YOU` have to activate `source`'s ability, or to cast `source`.
fn option_of(g: &Game, r: &Request, source: ObjectId) -> Option<ActionOption> {
    match &r.kind {
        RequestKind::Priority { options } => options.iter().find(|o| o.source == Some(source)).cloned(),
        RequestKind::PayCosts { options, .. } | RequestKind::UnlessPays { options, .. } => options.iter().find(|o| o.source == Some(source)).cloned(),
        _ => None,
    }
    .filter(|_| r.player == YOU && !g.is_over())
}

/// Both players pass until the stack is empty, answering nothing else.
fn resolve_all(g: &mut Game) -> Request {
    loop {
        let r = next(g);
        if g.state.stack.is_empty() || !matches!(r.kind, RequestKind::Priority { .. }) {
            return r;
        }
        answer(g, &r, Answer::Pass);
    }
}

#[test]
fn an_activated_ability_beholds_a_permanent_once_its_mana_is_paid_and_not_without_one() {
    let mut g = game();
    let seer = put(&mut g, &SEER, Zone::BATTLEFIELD);
    add_mana(&mut g, 1);
    // Nothing to behold: not offered (CR 602.2b, 118.3).
    assert!(!priority_options(&mut cx(&mut g), YOU).iter().any(|o| o.source == Some(seer)));
    let bear = put(&mut g, &BEAR, Zone::BATTLEFIELD);
    let r = next(&mut g);
    let option = option_of(&g, &r, seer).expect("the Seer's ability is offered");
    answer(&mut g, &r, Answer::Act(Action::of(&option)));
    let mut r = next(&mut g);
    if let RequestKind::PayCosts { options, .. } = &r.kind {
        let confirm = options.iter().find(|o| o.kind == ActionKind::ConfirmCasting).expect("the pool pays it");
        answer(&mut g, &r, Answer::Act(Action::of(confirm)));
        r = next(&mut g);
    }
    match &r.kind {
        RequestKind::ChooseCards { purpose: Purpose::Behold, .. } => {
            assert_eq!(crate::first_choices(&g, &r), vec![bear]);
            assert_eq!(g.state.player(YOU).pool.total(), 0, "the mana is paid before the behold is chosen");
        }
        other => panic!("expected the behold, got {other:?}"),
    }
    answer(&mut g, &r, Answer::Cards(vec![bear]));
    let life = g.state.player(YOU).life;
    resolve_all(&mut g);
    assert_eq!(g.state.player(YOU).life, life + 3);
    // The Bear was only chosen: it is still there (CR 701.4a).
    assert!(g.state.obj(bear).zone.is(ZoneKind::Battlefield));
}

#[test]
fn an_activated_ability_may_behold_its_own_source() {
    let mut g = game();
    let gazer = put(&mut g, &GAZER, Zone::BATTLEFIELD);
    let r = next(&mut g);
    let option = option_of(&g, &r, gazer).expect("the Gazer may behold itself");
    answer(&mut g, &r, Answer::Act(Action::of(&option)));
    let r = next(&mut g);
    match &r.kind {
        RequestKind::ChooseCards { purpose: Purpose::Behold, .. } => assert_eq!(crate::first_choices(&g, &r), vec![gazer]),
        other => panic!("expected the behold, got {other:?}"),
    }
    answer(&mut g, &r, Answer::Cards(vec![gazer]));
    let life = g.state.player(YOU).life;
    resolve_all(&mut g);
    assert_eq!(g.state.player(YOU).life, life + 3);
}

/// A card in the hand is revealed to behold it (CR 701.4a).
#[test]
fn an_activated_ability_beholds_a_card_from_the_hand_by_revealing_it() {
    let mut g = game();
    let seer = put(&mut g, &SEER, Zone::BATTLEFIELD);
    let bear = put(&mut g, &BEAR, Zone::hand(YOU));
    add_mana(&mut g, 1);
    let r = next(&mut g);
    let activate = option_of(&g, &r, seer).expect("offered");
    answer(&mut g, &r, Answer::Act(Action::of(&activate)));
    let mut r = next(&mut g);
    if let RequestKind::PayCosts { options, .. } = &r.kind {
        let confirm = options.iter().find(|o| o.kind == ActionKind::ConfirmCasting).expect("the pool pays it");
        answer(&mut g, &r, Answer::Act(Action::of(confirm)));
        r = next(&mut g);
    }
    answer(&mut g, &r, Answer::Cards(vec![bear]));
    assert!(g.log.iter().any(|rec| matches!(&rec.event, Event::Revealed { card, .. } if *card == bear)), "the Bear was revealed");
    // It stays revealed until the ability leaves the stack (CR 701.20a): one of the objects its cost chose.
    let item = *g.state.stack.last().expect("the ability is on the stack");
    assert_eq!(g.state.members(item.members), &[bear]);
    let cx = crate::Cx { g: &mut g, frame: usize::MAX, cursor: 0, mana_source: None, entering: None, lineage: Vec::new(), asked_at: None, doer: None, producing: None };
    assert!(crate::ops::seen_by_all(&cx, bear, false), "the Bear is public while the ability is on the stack");
}

/// The Returner's {1} is paid by tapping the Dork for mana while it is activated (CR 605.3a, 601.2g); the tapped creature
/// it returns is chosen after that (CR 601.2h), so the Dork it just tapped is one.
#[test]
fn a_mana_ability_returns_a_tapped_creature_chosen_once_its_own_mana_is_paid() {
    let mut g = game();
    let returner = put(&mut g, &RETURNER, Zone::BATTLEFIELD);
    // No creature at all: not offered.
    assert!(!priority_options(&mut cx(&mut g), YOU).iter().any(|o| o.source == Some(returner)));
    let dork = put(&mut g, &DORK, Zone::BATTLEFIELD);
    let r = next(&mut g);
    let option = option_of(&g, &r, returner).expect("the Dork could be tapped for its mana: offered");
    assert!(option.target_candidates.is_empty(), "the tapped creature is not chosen with the activation");
    answer(&mut g, &r, Answer::Act(Action::of(&option)));
    let r = next(&mut g);
    let tap_dork = option_of(&g, &r, dork).expect("the Dork's mana ability is offered while paying");
    answer(&mut g, &r, Answer::Act(Action::of(&tap_dork)));
    let mut r = next(&mut g);
    if let RequestKind::PayCosts { options, .. } = &r.kind {
        let confirm = options.iter().find(|o| o.kind == ActionKind::ConfirmCasting).expect("the pool pays it");
        answer(&mut g, &r, Answer::Act(Action::of(confirm)));
        r = next(&mut g);
    }
    match &r.kind {
        RequestKind::ChooseCards { purpose: Purpose::ReturnToHand, .. } => assert_eq!(crate::first_choices(&g, &r), vec![dork]),
        other => panic!("expected the tapped creature, got {other:?}"),
    }
    answer(&mut g, &r, Answer::Cards(vec![dork]));
    next(&mut g);
    assert!(g.state.obj(dork).became.is_some_and(|d| g.state.obj(d).zone == Zone::hand(YOU)), "the Dork went back to the hand");
    assert_eq!(g.state.player(YOU).pool.get(Mana::G), 3);
}

/// Casts `spell` (free: it has no mana cost) and passes until it resolves, giving back the payment question.
fn cast_to_payment(g: &mut Game, spell: ObjectId) -> Request {
    let r = next(g);
    let cast = option_of(g, &r, spell).expect("the spell is offered");
    answer(g, &r, Answer::Act(Action::of(&cast)));
    let r = resolve_all(g);
    assert!(matches!(r.kind, RequestKind::UnlessPays { .. }), "expected the payment, got {:?}", r.kind);
    r
}

fn pays(r: &Request) -> Option<ActionOption> {
    match &r.kind {
        RequestKind::UnlessPays { options, .. } => options.iter().find(|o| o.kind == ActionKind::PayCost).cloned(),
        _ => None,
    }
}

#[test]
fn resolution_life_payment_above_u16_is_exact() {
    let mut g = game();
    g.state.update_player(YOU, |p| p.life = 65_535);
    let spell = put(&mut g, &HIGH_LIFE, Zone::hand(YOU));
    let r = next(&mut g);
    let cast = option_of(&g, &r, spell).expect("the spell is offered");
    answer(&mut g, &r, Answer::Act(Action::of(&cast)));
    let r = resolve_all(&mut g);
    assert!(matches!(r.kind, RequestKind::Priority { .. }), "65,535 life cannot pay 65,536");
    assert_eq!(g.state.player(YOU).life, 65_535);

    let mut g = game();
    g.state.update_player(YOU, |p| p.life = i32::MAX);
    let spell = put(&mut g, &IMPOSSIBLE_LIFE, Zone::hand(YOU));
    let r = next(&mut g);
    let cast = option_of(&g, &r, spell).expect("the spell is offered");
    answer(&mut g, &r, Answer::Act(Action::of(&cast)));
    let r = resolve_all(&mut g);
    assert!(matches!(r.kind, RequestKind::Priority { .. }), "a life cost above i32::MAX cannot be paid");
    assert_eq!(g.state.player(YOU).life, i32::MAX);

    let mut g = game();
    g.state.update_player(YOU, |p| p.life = 65_536);
    let spell = put(&mut g, &HIGH_LIFE, Zone::hand(YOU));
    let r = cast_to_payment(&mut g, spell);
    assert!(matches!(r.kind, RequestKind::UnlessPays { life: 65_536, .. }));
    let pay = pays(&r).expect("65,536 life pays the full cost");
    answer(&mut g, &r, Answer::Act(Action::of(&pay)));
    next(&mut g);
    assert_eq!(g.state.player(YOU).life, 5, "full cost paid before the effect gains five life");
}

#[test]
fn a_resolving_spell_takes_mana_and_a_sacrificed_creature_chosen_as_it_is_paid() {
    // No mana: only declining.
    let mut g = game();
    let offer = put(&mut g, &OFFER, Zone::hand(YOU));
    put(&mut g, &BEAR, Zone::BATTLEFIELD);
    let r = cast_to_payment(&mut g, offer);
    assert!(pays(&r).is_none());
    // No creature: only declining.
    let mut g = game();
    let offer = put(&mut g, &OFFER, Zone::hand(YOU));
    add_mana(&mut g, 1);
    let r = cast_to_payment(&mut g, offer);
    assert!(pays(&r).is_none());

    let mut g = game();
    let offer = put(&mut g, &OFFER, Zone::hand(YOU));
    let bear = put(&mut g, &BEAR, Zone::BATTLEFIELD);
    add_mana(&mut g, 1);
    let r = cast_to_payment(&mut g, offer);
    let pay = pays(&r).expect("the pool and the Bear pay it");
    answer(&mut g, &r, Answer::Act(Action::of(&pay)));
    let r = next(&mut g);
    match &r.kind {
        RequestKind::ChooseCards { purpose: Purpose::SacrificeCost, .. } => assert_eq!(crate::first_choices(&g, &r), vec![bear]),
        other => panic!("expected the sacrifice, got {other:?}"),
    }
    let life = g.state.player(YOU).life;
    answer(&mut g, &r, Answer::Cards(vec![bear]));
    next(&mut g);
    assert!(g.state.zone(Zone::graveyard(YOU)).iter().any(|&c| g.state.obj(c).card == g.cards.id_of(&BEAR)), "the Bear was sacrificed");
    assert_eq!(g.state.player(YOU).life, life + 5);
    assert_eq!(g.state.player(YOU).pool.total(), 0);
}

/// "Return a tapped creature" during a resolution, the Dork untapped: paying is offered once its mana ability has tapped
/// it for the {1} (CR 605.3a), and the Dork is then the tapped creature returned.
#[test]
fn a_resolving_spells_tapped_creature_is_judged_after_the_mana_abilities() {
    let mut g = game();
    let recall = put(&mut g, &RECALL, Zone::hand(YOU));
    let dork = put(&mut g, &DORK, Zone::BATTLEFIELD);
    let r = cast_to_payment(&mut g, recall);
    assert!(pays(&r).is_none(), "nothing tapped and no mana yet");
    let tap = option_of(&g, &r, dork).expect("the Dork's mana ability is offered");
    answer(&mut g, &r, Answer::Act(Action::of(&tap)));
    let r = next(&mut g);
    let pay = pays(&r).expect("the Dork is tapped and its {G} pays the {1}");
    answer(&mut g, &r, Answer::Act(Action::of(&pay)));
    let r = next(&mut g);
    match &r.kind {
        RequestKind::ChooseCards { purpose: Purpose::ReturnToHand, .. } => assert_eq!(crate::first_choices(&g, &r), vec![dork]),
        other => panic!("expected the tapped creature, got {other:?}"),
    }
    let life = g.state.player(YOU).life;
    answer(&mut g, &r, Answer::Cards(vec![dork]));
    next(&mut g);
    assert_eq!(g.state.player(YOU).life, life + 5);
    assert!(g.state.zone(Zone::hand(YOU)).iter().any(|&c| g.state.obj(c).card == g.cards.id_of(&DORK)));
}

/// A discard and energy: offered only with a card and the energy; the card chosen is discarded and the energy paid. A
/// choice given up is undone, and the payer asked again.
#[test]
fn a_resolving_spell_takes_a_discard_and_energy_and_a_choice_given_up_asks_again() {
    let mut g = game();
    let tithe = put(&mut g, &TITHE, Zone::hand(YOU));
    let card = put(&mut g, &BEAR, Zone::hand(YOU));
    let r = cast_to_payment(&mut g, tithe);
    assert!(pays(&r).is_none(), "no energy");

    let mut g = game();
    let tithe = put(&mut g, &TITHE, Zone::hand(YOU));
    let card2 = put(&mut g, &BEAR, Zone::hand(YOU));
    let _ = card;
    crate::ops::change_energy(&mut cx(&mut g), YOU, 2, YOU);
    let r = cast_to_payment(&mut g, tithe);
    let pay = pays(&r).expect("a card and the energy");
    answer(&mut g, &r, Answer::Act(Action::of(&pay)));
    let r = next(&mut g);
    assert!(matches!(r.kind, RequestKind::ChooseCards { purpose: Purpose::Discard, .. }));
    // Given up: asked to pay again.
    answer(&mut g, &r, Answer::Cancel);
    let r = next(&mut g);
    let pay = pays(&r).expect("asked again");
    answer(&mut g, &r, Answer::Act(Action::of(&pay)));
    let r = next(&mut g);
    let life = g.state.player(YOU).life;
    answer(&mut g, &r, Answer::Cards(vec![card2]));
    next(&mut g);
    assert_eq!(g.state.player(YOU).life, life + 5);
    assert_eq!(g.state.player(YOU).energy(), 1);
    assert!(g.state.zone(Zone::graveyard(YOU)).iter().any(|&c| g.state.obj(c).card == g.cards.id_of(&BEAR)));
}

/// A loyalty cost is no resolution's: never paid, and nothing is asked.
#[test]
fn a_resolving_spell_never_pays_a_part_only_an_activation_has() {
    let mut g = game();
    let vow = put(&mut g, &VOW, Zone::hand(YOU));
    let r = next(&mut g);
    let cast = option_of(&g, &r, vow).expect("offered");
    answer(&mut g, &r, Answer::Act(Action::of(&cast)));
    let life = g.state.player(YOU).life;
    let r = resolve_all(&mut g);
    assert!(matches!(r.kind, RequestKind::Priority { .. }) && g.state.stack.is_empty());
    assert_eq!(g.state.player(YOU).life, life);
}

/// "{T}, Return a tapped creature you control": the parts are paid in any order (CR 601.2h by way of 602.2b), so {T} taps
/// the Recoiler and it is the tapped creature returned - offered with no other creature, and the only one to choose.
#[test]
fn a_cost_with_its_own_tap_returns_its_source_as_the_tapped_creature() {
    let mut g = game();
    let recoiler = put(&mut g, &RECOILER, Zone::BATTLEFIELD);
    let r = next(&mut g);
    let option = option_of(&g, &r, recoiler).expect("offered: its own {T} taps it");
    answer(&mut g, &r, Answer::Act(Action::of(&option)));
    let r = next(&mut g);
    match &r.kind {
        RequestKind::ChooseCards { purpose: Purpose::ReturnToHand, .. } => assert_eq!(crate::first_choices(&g, &r), vec![recoiler]),
        other => panic!("expected the tapped creature, got {other:?}"),
    }
    answer(&mut g, &r, Answer::Cards(vec![recoiler]));
    let life = g.state.player(YOU).life;
    resolve_all(&mut g);
    assert_eq!(g.state.player(YOU).life, life + 3);
    assert!(g.state.zone(Zone::hand(YOU)).iter().any(|&c| g.state.obj(c).card == g.cards.id_of(&RECOILER)), "it went back to the hand");
}

/// A card spliced onto a spell is revealed from the spell's announcement until it leaves the stack (CR 701.20a, 702.47a),
/// before it has become cast (CR 601.2i) as much as after: a mana ability activated while the spell's cost is paid (CR
/// 601.2g) puts the hand into the library as the Librarian dies, and the Librarian, judged as it was just before (CR
/// 603.10a), triggers for the spliced card, one all players could see. Not spliced, every card it moves is hidden.
#[test]
fn a_card_revealed_for_a_spell_being_cast_is_looked_back_on_as_a_mana_ability_moves_it() {
    for spliced in [true, false] {
        let mut g = game();
        let librarian = put(&mut g, &LIBRARIAN, Zone::BATTLEFIELD);
        let stower = put(&mut g, &STOWER, Zone::BATTLEFIELD);
        let rite = put(&mut g, &RITE, Zone::hand(YOU));
        let graft = put(&mut g, &GRAFT, Zone::hand(YOU));
        add_mana(&mut g, if spliced { 1 } else { 0 });
        let r = next(&mut g);
        let cast = option_of(&g, &r, rite).expect("the Rite may be cast");
        answer(&mut g, &r, Answer::Act(Action::of(&cast)));
        let r = next(&mut g);
        assert!(matches!(r.kind, RequestKind::ChooseCards { purpose: Purpose::Splice, .. }), "splice is offered: {:?}", r.kind);
        answer(&mut g, &r, Answer::Cards(if spliced { vec![graft] } else { Vec::new() }));
        let r = next(&mut g);
        let stow = option_of(&g, &r, stower).expect("the Stower's mana ability may pay");
        answer(&mut g, &r, Answer::Act(Action::of(&stow)));
        assert!(g.state.obj(graft).became.is_some_and(|c| g.state.obj(c).zone.is(ZoneKind::Library)), "the Graft is in the library");
        assert!(!g.state.battlefield().contains(&librarian));
        let triggered = g.state.triggers.iter().any(|t| t.source == librarian)
            || g.state.stack.iter().any(|i| i.kind == StackKind::Triggered && i.object == librarian);
        assert_eq!(triggered, spliced, "the Librarian looks back only on a card all players could see");
    }
}

/// A Phyrexian symbol in a cost paid as a spell resolves is paid with its color or 2 life, chosen immediately before the cost
/// is paid (CR 107.4f, 118.13b): with no mana, paying is offered and takes 2 life; with {B} the payer may pay it with the
/// mana instead; with neither {B} nor 2 life to pay, only declining is offered.
#[test]
fn a_phyrexian_symbol_in_a_cost_paid_as_a_spell_resolves_is_paid_with_its_color_or_two_life() {
    // No mana: 2 life.
    let mut g = game();
    let toll = put(&mut g, &TOLL, Zone::hand(YOU));
    let r = cast_to_payment(&mut g, toll);
    let pay = pays(&r).expect("2 life pays it");
    answer(&mut g, &r, Answer::Act(Action::of(&pay)));
    let r = next(&mut g);
    match &r.kind {
        RequestKind::ChooseModes { modes, at_most, .. } => {
            assert_eq!(modes, &vec![Label::PayLife(Color::Black)]);
            assert_eq!(*at_most, 1);
        }
        other => panic!("expected the choice of life for {{B/P}}, got {other:?}"),
    }
    answer(&mut g, &r, Answer::Modes(vec![0]));
    next(&mut g);
    assert_eq!(g.state.player(YOU).life, 20 - 2 + 5, "2 life paid, 5 gained");

    // {B} in the pool: paid with the mana, no life.
    let mut g = game();
    let toll = put(&mut g, &TOLL, Zone::hand(YOU));
    g.state.add_mana(YOU, Mana::B, 1, toll, None, ManaLasts::Step);
    let r = cast_to_payment(&mut g, toll);
    let pay = pays(&r).expect("{B} pays it");
    answer(&mut g, &r, Answer::Act(Action::of(&pay)));
    let r = next(&mut g);
    assert!(matches!(r.kind, RequestKind::ChooseModes { .. }), "how {{B/P}} is paid is asked: {:?}", r.kind);
    answer(&mut g, &r, Answer::Modes(Vec::new()));
    next(&mut g);
    assert_eq!(g.state.player(YOU).life, 25, "no life paid");
    assert_eq!(g.state.player(YOU).pool.get(Mana::B), 0, "the {{B}} was spent");

    // Neither {B} nor 2 life: only declining.
    let mut g = game();
    let toll = put(&mut g, &TOLL, Zone::hand(YOU));
    g.state.update_player(YOU, |p| p.life = 1);
    let r = cast_to_payment(&mut g, toll);
    assert!(pays(&r).is_none(), "a {{B/P}} with no {{B}} and 1 life can't be paid");
}

/// 300 monocolored hybrid symbols in a cost paid as a spell resolves, more than a choice of modes holds (255): which are
/// paid with {2} is asked a page at a time, 255 then 45, none of them lost to a count cut to a byte (CR 107.4e, 118.13b).
#[test]
fn more_monocolored_hybrid_symbols_than_a_choice_holds_are_asked_a_page_at_a_time() {
    let mut g = game();
    let hoard = put(&mut g, &HOARD, Zone::hand(YOU));
    let r = next(&mut g);
    let cast = option_of(&g, &r, hoard).expect("the spell is offered");
    answer(&mut g, &r, Answer::Act(Action::of(&cast)));
    let r = resolve_all(&mut g);
    let RequestKind::ChooseModes { modes, at_most, .. } = &r.kind else { panic!("which {{2/C}} to pay with {{2}}, not {:?}", r.kind) };
    assert_eq!((modes.len(), *at_most), (255, 255));
    assert!(modes.iter().all(|m| matches!(m, Label::PayGeneric(_))));
    answer(&mut g, &r, Answer::Modes(Vec::new()));
    let r = next(&mut g);
    let RequestKind::ChooseModes { modes, at_most, .. } = &r.kind else { panic!("the rest, not {:?}", r.kind) };
    assert_eq!((modes.len(), *at_most), (45, 45));
    answer(&mut g, &r, Answer::Modes(Vec::new()));
    let r = next(&mut g);
    assert!(matches!(r.kind, RequestKind::UnlessPays { .. }), "then the payment, not {:?}", r.kind);
}

/// A cost of 300 creatures sacrificed is paid in full: not a cost nobody can pay, and not a smaller one.
#[test]
fn a_sacrifice_cost_past_a_byte_is_paid_in_full() {
    let mut g = game();
    let offer = put(&mut g, &MASS_OFFER, Zone::hand(YOU));
    let bears: Vec<ObjectId> = (0..300).map(|_| put(&mut g, &BEAR, Zone::BATTLEFIELD)).collect();
    let r = cast_to_payment(&mut g, offer);
    let pay = pays(&r).expect("300 creatures pay 300 sacrificed");
    answer(&mut g, &r, Answer::Act(Action::of(&pay)));
    let r = next(&mut g);
    assert!(matches!(&r.kind, RequestKind::ChooseCards { purpose: Purpose::SacrificeCost, .. }));
    let life = g.state.player(YOU).life;
    answer(&mut g, &r, Answer::Cards(bears));
    next(&mut g);
    assert_eq!(g.state.zone(Zone::graveyard(YOU)).iter().filter(|&&c| g.state.obj(c).card == g.cards.id_of(&BEAR)).count(), 300);
    assert_eq!(g.state.player(YOU).life, life + 5);

    // One creature short is not payable.
    let mut g = game();
    let offer = put(&mut g, &MASS_OFFER, Zone::hand(YOU));
    for _ in 0..299 {
        put(&mut g, &BEAR, Zone::BATTLEFIELD);
    }
    let r = cast_to_payment(&mut g, offer);
    assert!(pays(&r).is_none(), "299 creatures cannot sacrifice 300");
}

/// A cost of 300 counters put on the creature is paid in full: 300 -1/-1 counters, as 300 age counters ask of Aboroth.
#[test]
fn a_counters_cost_past_a_byte_puts_every_counter() {
    let mut g = game();
    let witherer = put(&mut g, &WITHERER, Zone::BATTLEFIELD);
    let r = next(&mut g);
    let option = option_of(&g, &r, witherer).expect("300 counters can be put on it");
    answer(&mut g, &r, Answer::Act(Action::of(&option)));
    resolve_all(&mut g);
    assert_eq!(g.state.obj(witherer).counters(CounterKind::MinusOne), 300);
}
