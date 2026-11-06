use crate::{Cx, Game, Setup};
use cantrip_api::*;
use cantrip_state::{Frame, FrameKind};

const YOU: PlayerId = PlayerId(0);
const COUNTER_COST: Cost = Cost { variable: Some(VariableCost::RemoveAnyNumberOfCounters(CounterKind::Charge)), ..Cost::NONE };

fn add_x(g: &mut dyn cantrip_api::Game, asker: &Asker, _: Option<Color>) -> Result<(), Suspend> {
    g.add_mana(asker.controller, Mana::G, g.mana_ability_x().unwrap_or(0));
    Ok(())
}
fn add_one_plus_x(g: &mut dyn cantrip_api::Game, asker: &Asker, _: Option<Color>) -> Result<(), Suspend> {
    g.add_mana(asker.controller, Mana::G, 1 + g.mana_ability_x().unwrap_or(0));
    Ok(())
}
fn nothing(_: &mut dyn cantrip_api::Game, _: &StackItem) -> Result<(), Suspend> { Ok(()) }

static FILLER: CardDef = CardDef { name: "Mana cost filler", types: TypeSet::of(&[CardType::Land]), ..CardDef::BLANK };
static STORE: CardDef = CardDef {
    name: "Mana counter store", types: TypeSet::of(&[CardType::Land]),
    abilities: &[mana_ability(TAP.and(COUNTER_COST), add_by(add_x), "{T}, Remove any number of charge counters: Add that much {G}.").x_is_mana_amount()],
    ..CardDef::BLANK
};
static BATTERY: CardDef = CardDef {
    name: "Mana counter battery", types: TypeSet::of(&[CardType::Artifact]),
    abilities: &[mana_ability(TAP.and(COUNTER_COST), add_by(add_one_plus_x), "{T}, Remove any number of charge counters: Add {G} and that much more.").x_is_mana_amount()],
    ..CardDef::BLANK
};
static REPEAT: CardDef = CardDef {
    name: "Mana counter repeat", abilities: &[mana_ability(COUNTER_COST, add_by(add_x), "Remove any number of charge counters: Add that much {G}.").x_is_mana_amount()],
    ..BATTERY
};
static SACRIFICE: CardDef = CardDef {
    name: "Mana counter sacrifice", abilities: &[mana_ability(COUNTER_COST.and(SACRIFICE_THIS), add_by(add_x), "Remove any number of charge counters, sacrifice this: Add that much {G}.").x_is_mana_amount()],
    ..BATTERY
};
static EXILE: CardDef = CardDef {
    name: "Mana counter exile", abilities: &[mana_ability(COUNTER_COST.and(Cost { exile_self: true, ..Cost::NONE }), add_by(add_x), "Remove any number of charge counters, exile this: Add that much {G}.").x_is_mana_amount()],
    ..BATTERY
};
static FIXED: CardDef = CardDef {
    name: "Mana counter fixed", abilities: &[mana_ability(COUNTER_COST.and(remove_counters(CounterKind::Charge, 2)), add_by(add_x), "Remove two charge counters and any number more: Add that much {G}.").x_is_mana_amount()],
    ..BATTERY
};
fn two_mana_from_a_land(q: &dyn Query, _: &Asker, proposal: &Proposal) -> bool {
    matches!(proposal, Proposal::ManaProduction(p) if p.total() >= 2 && q.chars(p.permanent).is(CardType::Land))
}
fn colorless(_: &dyn Query, _: &Asker, proposal: &mut Proposal) {
    if let Proposal::ManaProduction(p) = proposal { p.mana = ManaPool::default(); p.mana.0[Mana::C as usize] = 1; }
}
static SPHERE: CardDef = CardDef {
    name: "Mana counter Sphere", types: TypeSet::of(&[CardType::Artifact]),
    replacements: &[replacement(ReplacementKind::ManaProduction, two_mana_from_a_land, colorless, "produces {C} instead")],
    ..CardDef::BLANK
};
static PAID: CardDef = CardDef {
    name: "Mana counter paid", abilities: &[mana_ability(COUNTER_COST.and(cost("{X}, {T}")), add_by(add_x), "{X}, {T}, Remove X charge counters: Add X {G}.").x_is_mana_amount()],
    ..BATTERY
};
static MANA_X: CardDef = CardDef {
    name: "Mana own X", abilities: &[mana_ability(cost("{X}, {T}"), add_by(add_x), "{X}, {T}: Add X {G}.").x_is_mana_amount()],
    ..BATTERY
};
static SPELL: CardDef = CardDef {
    name: "Mana cost X spell", cost: mana("{X}"), has_cost: true, types: TypeSet::of(&[CardType::Sorcery]),
    spell: spell(&[], nothing), ..CardDef::BLANK
};

fn game() -> Game {
    let table = Box::leak(Box::new(CardTable::new([&FILLER, &STORE, &BATTERY, &REPEAT, &SACRIFICE, &EXILE, &FIXED, &SPHERE, &PAID, &MANA_X, &SPELL])));
    let deck = vec![table.id_of(&FILLER); 20];
    let mut game = Game::new(table, &Setup { decks: vec![deck.clone(), deck], ..Default::default() });
    game.verify = true;
    while !game.state.agenda.is_empty() { game.state.frame_remove(0); }
    game.state.update_turn(|t| { t.active = YOU; t.priority = YOU; t.set_step(Step::PrecombatMain); });
    game.state.frame_push(Frame::new(FrameKind::Priority));
    game.state.update_player(YOU, |p| p.turn_began = 1);
    game
}
fn put(game: &mut Game, def: &'static CardDef, zone: Zone) -> ObjectId {
    let id = game.state.push_obj(Obj::new(game.cards.id_of(def), YOU, zone));
    game.state.zone_push(zone, id);
    game.state.update_obj(id, |o| o.x = 9);
    if zone.is(ZoneKind::Battlefield) {
        let mut cx = Cx { g: game, frame: usize::MAX, cursor: 0, mana_source: None, entering: None, lineage: Vec::new(), damage: None, asked_at: None, doer: None, producing: None };
        crate::counters::put(&mut cx, id, CounterKind::Charge, 5, YOU, crate::counters::How::Effect);
    }
    game.state.see_statics();
    id
}
fn next(game: &mut Game) -> Request { game.run(); game.pending().cloned().expect("a request") }
fn answer(game: &mut Game, answer: Answer) {
    let request = game.pending().cloned().expect("a request");
    game.answer(request.id, request.player, answer).expect("legal answer");
    game.run();
}
fn act(game: &mut Game, source: ObjectId) {
    let request = next(game);
    let options = match &request.kind {
        RequestKind::Priority { options } | RequestKind::PayCosts { options, .. } => options,
        other => panic!("expected an action window: {other:?}"),
    };
    let option = options.iter().find(|o| o.source == Some(source)).expect("ability or spell offered");
    let action = Action::of(option);
    answer(game, Answer::Act(action));
}
fn announce(game: &mut Game, value: u32, most: u32) {
    assert!(matches!(game.pending().map(|r| &r.kind), Some(RequestKind::PayCosts { announcing: true, most: m, .. }) if *m == most), "expected X bounded by {most}: {:?}", game.pending());
    answer(game, Answer::Announce(value));
    finish_cost_order(game);
}
fn finish_cost_order(game: &mut Game) {
    while let Some(Request { kind: RequestKind::ChooseCostPayment { choices, .. }, .. }) = game.pending() {
        let chosen = choices.iter().position(|c| c.kind == CostPaymentKind::Tap)
            .or_else(|| choices.iter().position(|c| c.kind == CostPaymentKind::Mana))
            .or_else(|| choices.iter().position(|c| c.kind == CostPaymentKind::Counters)).unwrap_or(0);
        answer(game, Answer::CostPayment(chosen));
    }
}
fn finish_payment(game: &mut Game) {
    if let Some(Request { kind: RequestKind::PayCosts { announcing: false, options, .. }, .. }) = game.pending() {
        let option = options.iter().find(|o| o.kind == ActionKind::ConfirmCasting).expect("pool pays");
        let action = Action::of(option); answer(game, Answer::Act(action));
        finish_cost_order(game);
    }
}

#[test]
fn a_counter_mana_ability_announces_pays_and_resolves_immediately() {
    let mut game = game(); let source = put(&mut game, &STORE, Zone::BATTLEFIELD);
    act(&mut game, source);
    assert!(!game.state.obj(source).tapped);
    announce(&mut game, 2, 5);
    assert_eq!(game.state.obj(source).counters(CounterKind::Charge), 3);
    assert_eq!(game.state.player(YOU).pool.get(Mana::G), 2);
    assert_eq!(game.state.obj(source).x, 9);
    assert!(game.state.stack.is_empty()); assert!(game.state.obj(source).tapped);
}

#[test]
fn zero_counters_still_allows_a_battery_to_add_its_fixed_mana() {
    let mut game = game(); let source = put(&mut game, &BATTERY, Zone::BATTLEFIELD);
    game.state.update_obj(source, |o| o.remove_counters(CounterKind::Charge, 5));
    act(&mut game, source); announce(&mut game, 0, 0);
    assert_eq!(game.state.player(YOU).pool.get(Mana::G), 1);
    assert!(game.state.obj(source).tapped);
}

#[test]
fn announcing_too_many_counters_is_rejected_without_paying() {
    let mut game = game(); let source = put(&mut game, &STORE, Zone::BATTLEFIELD);
    act(&mut game, source); let request = game.pending().cloned().unwrap();
    assert!(game.answer(request.id, request.player, Answer::Announce(6)).is_err());
    assert_eq!(game.state.obj(source).counters(CounterKind::Charge), 5);
    assert!(!game.state.obj(source).tapped);
}

#[test]
fn cancelling_the_announcement_pays_no_cost() {
    let mut game = game(); let source = put(&mut game, &STORE, Zone::BATTLEFIELD);
    act(&mut game, source);
    let option = match &game.pending().unwrap().kind {
        RequestKind::PayCosts { announcing: true, options, .. } => options.iter().find(|o| o.kind == ActionKind::CancelCasting).unwrap(),
        other => panic!("expected announcement: {other:?}"),
    };
    let action = Action::of(option); answer(&mut game, Answer::Act(action));
    assert_eq!(game.state.obj(source).counters(CounterKind::Charge), 5);
    assert!(!game.state.obj(source).tapped); assert_eq!(game.state.obj(source).x, 9);
    assert!(!game.log.iter().any(|r| matches!(r.event, Event::AbilityActivated { source: s, .. } if s == source)));
}

#[test]
fn repeated_activations_each_have_their_own_x() {
    let mut game = game(); let source = put(&mut game, &REPEAT, Zone::BATTLEFIELD);
    act(&mut game, source); announce(&mut game, 2, 5);
    act(&mut game, source); announce(&mut game, 1, 3);
    assert_eq!(game.state.player(YOU).pool.get(Mana::G), 3);
    assert_eq!(game.state.obj(source).counters(CounterKind::Charge), 2);
    assert_eq!(game.state.obj(source).x, 9);
}

#[test]
fn paying_with_source_sacrifice_keeps_the_activation_x() {
    let mut game = game(); let source = put(&mut game, &SACRIFICE, Zone::BATTLEFIELD);
    act(&mut game, source); announce(&mut game, 2, 5);
    assert!(game.state.obj(source).became.is_some());
    assert_eq!(game.state.player(YOU).pool.get(Mana::G), 2);
    assert_eq!(game.state.obj(source).x, 9);
}

#[test]
fn mana_and_counter_costs_use_one_announcement() {
    let mut game = game(); let source = put(&mut game, &PAID, Zone::BATTLEFIELD);
    game.state.add_mana(YOU, Mana::C, 2, source, None, ManaLasts::Step, false);
    act(&mut game, source); announce(&mut game, 2, 5); finish_payment(&mut game);
    assert_eq!(game.state.player(YOU).pool.get(Mana::G), 2);
    assert_eq!(game.state.player(YOU).pool.get(Mana::C), 0);
    assert_eq!(game.state.obj(source).counters(CounterKind::Charge), 3);
    assert_eq!(game.state.obj(source).x, 9);
}

#[test]
fn a_nested_mana_activation_restores_the_outer_activations_different_x() {
    let mut game = game();
    let outer = put(&mut game, &PAID, Zone::BATTLEFIELD);
    let inner = put(&mut game, &STORE, Zone::BATTLEFIELD);
    game.state.add_mana(YOU, Mana::C, 1, outer, None, ManaLasts::Step, false);
    act(&mut game, outer); announce(&mut game, 3, 5);
    act(&mut game, inner); announce(&mut game, 2, 5); finish_payment(&mut game);
    assert_eq!(game.state.player(YOU).pool.get(Mana::G), 3);
    assert_eq!(game.state.player(YOU).pool.get(Mana::C), 0);
    assert_eq!(game.state.obj(outer).counters(CounterKind::Charge), 2);
    assert_eq!(game.state.obj(inner).counters(CounterKind::Charge), 3);
    assert_eq!(game.state.obj(outer).x, 9);
    assert_eq!(game.state.obj(inner).x, 9);
    assert_eq!(game.mana_activation_x, None);
}

#[test]
fn a_mana_symbol_x_is_not_written_into_the_permanent() {
    let mut game = game(); let source = put(&mut game, &MANA_X, Zone::BATTLEFIELD);
    game.state.add_mana(YOU, Mana::C, 2, source, None, ManaLasts::Step, false);
    act(&mut game, source); announce(&mut game, 2, u32::MAX); finish_payment(&mut game);
    assert_eq!(game.state.player(YOU).pool.get(Mana::G), 2);
    assert_eq!(game.state.player(YOU).pool.get(Mana::C), 0);
    assert_eq!(game.state.obj(source).x, 9);
}

#[test]
fn mana_activated_during_payment_does_not_replace_the_parent_spells_x() {
    let mut game = game(); let source = put(&mut game, &BATTERY, Zone::BATTLEFIELD);
    let spell = put(&mut game, &SPELL, Zone::hand(YOU));
    act(&mut game, spell); announce(&mut game, 3, 1000);
    act(&mut game, source); announce(&mut game, 2, 5); finish_payment(&mut game);
    assert_eq!(game.state.player(YOU).pool.total(), 0);
    assert_eq!(game.state.stack.last().unwrap().x, 3);
    assert_eq!(game.state.obj(source).x, 9);
    assert_eq!(game.state.obj(source).counters(CounterKind::Charge), 3);
}

#[test]
fn paying_counters_before_exiling_the_source_keeps_the_activation_x() {
    let mut game = game(); let source = put(&mut game, &EXILE, Zone::BATTLEFIELD);
    act(&mut game, source); announce(&mut game, 2, 5);
    let moved = game.state.obj(source).became.expect("exiled as a cost");
    assert!(game.state.obj(moved).zone.is(ZoneKind::Exile));
    assert_eq!(game.state.obj(source).counters(CounterKind::Charge), 3);
    assert_eq!(game.state.player(YOU).pool.get(Mana::G), 2);
    assert_eq!(game.state.obj(source).x, 9);
}

#[test]
fn fixed_and_variable_removal_cannot_spend_the_same_counters_twice() {
    let mut game = game(); let source = put(&mut game, &FIXED, Zone::BATTLEFIELD);
    act(&mut game, source); announce(&mut game, 3, 3);
    assert_eq!(game.state.obj(source).counters(CounterKind::Charge), 0);
    assert_eq!(game.state.player(YOU).pool.get(Mana::G), 3);
}

#[test]
fn could_produce_ignores_counter_cost_payability_and_includes_the_two_mana_replacement() {
    let mut game = game(); let source = put(&mut game, &STORE, Zone::BATTLEFIELD);
    game.state.update_obj(source, |o| o.remove_counters(CounterKind::Charge, 5));
    let types = |game: &mut Game| {
        let cx = Cx { g: game, frame: usize::MAX, cursor: 0, mana_source: None, entering: None, lineage: Vec::new(), damage: None, asked_at: None, doer: None, producing: None };
        crate::actions::producible_types(&cx, source)
    };
    assert_eq!(types(&mut game), (ColorSet::of(Color::Green), false));
    put(&mut game, &SPHERE, Zone::BATTLEFIELD);
    assert_eq!(types(&mut game), (ColorSet::of(Color::Green), true));
    assert_eq!(game.state.obj(source).counters(CounterKind::Charge), 0);
    assert_eq!(game.state.obj(source).x, 9);
    assert_eq!(game.state.player(YOU).pool.total(), 0);
}
