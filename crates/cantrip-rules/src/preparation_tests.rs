//! Preparation is consumed at cast completion, not announcement or payment (CR 722.3c, 601.2a-i). Test-only spells
//! have costs and cast-time observations that inspect the designation, so an early consumption cannot pass unnoticed.

use crate::{Cx, Game, Setup, ops};
use cantrip_api::*;

const YOU: PlayerId = PlayerId(0);
const THEM: PlayerId = PlayerId(1);

fn nothing(_: &mut dyn cantrip_api::Game, _: &StackItem) -> Result<(), Suspend> { Ok(()) }

fn any_prepared(q: &dyn Query) -> bool {
    q.state().battlefield().iter().any(|&id| q.obj(id).prepared)
}

fn while_prepared(q: &dyn Query, _: &Asker, _: ObjectId) -> bool { any_prepared(q) }
fn prepared_source(q: &dyn Query, asker: &Asker) -> bool { q.obj(asker.source).prepared }
fn prepared_as_cast(q: &dyn Query, _: &Asker) -> u32 { u32::from(any_prepared(q)) }

fn cast_after_unpreparing(q: &dyn Query, _: ObjectId, event: &Event) -> Option<Fired> {
    match event {
        Event::SpellCast { .. } if !any_prepared(q) => Fired::PLAIN,
        _ => None,
    }
}

static SPELL: CardDef = CardDef {
    name: "Preparation Test Spell",
    cost: mana("{1}{U}"),
    types: TypeSet::of(&[CardType::Sorcery]),
    additional_cost: Some(discard(1, while_prepared)),
    cast_fact: Some(prepared_as_cast),
    spell: spell(&[], nothing),
    ..CardDef::BLANK
};
static SOURCE: CardDef = CardDef {
    name: "Preparation Test Source",
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(2), toughness: Some(3),
    prepare: Some(&SPELL),
    abilities: &[mana_ability(TAP, add(Mana::U), "{T}: Add {U}. Activate only while prepared.").condition(prepared_source)],
    ..CardDef::BLANK
};
static LAND: CardDef = CardDef {
    name: "Preparation Test Land", has_cost: false,
    types: TypeSet::of(&[CardType::Land]),
    abilities: &[mana_ability(TAP, add(Mana::U), "{T}: Add {U}.")],
    ..CardDef::BLANK
};

fn blink_prepared(g: &mut dyn cantrip_api::Game, asker: &Asker, _: Option<Color>) -> Result<(), Suspend> {
    let source = g.state().battlefield().iter().copied().find(|&id| g.obj(id).prepared).unwrap();
    let exiled = g.exile_all(&[source])[0];
    let returned = g.put_onto_battlefield(exiled, asker.controller, false)?;
    g.become_prepared(returned);
    g.add_mana(asker.controller, Mana::U, 1);
    Ok(())
}

static BLINK_LAND: CardDef = CardDef {
    abilities: &[mana_ability(TAP, add(Mana::U).by(blink_prepared), "{T}: Blink a prepared source and add {U}.")],
    ..LAND
};
static WATCH: CardDef = CardDef {
    name: "Preparation Test Watch", types: TypeSet::of(&[CardType::Enchantment]),
    abilities: &[triggered(when(EventMask::SPELL_CAST, cast_after_unpreparing), nothing, "Whenever a spell is cast while nothing is prepared, ...")],
    ..CardDef::BLANK
};

fn cx(game: &mut Game) -> Cx<'_> {
    Cx { g: game, frame: usize::MAX, cursor: 0, mana_source: None, entering: None, lineage: Vec::new(), asked_at: None, doer: None, producing: None }
}

fn refresh_priority(game: &mut Game) {
    game.state.frame_pop();
    game.state.frame_push(cantrip_state::Frame::new(cantrip_state::FrameKind::Priority));
    game.run();
}

fn game() -> (Game, ObjectId, ObjectId, ObjectId) {
    let table = Box::leak(Box::new(CardTable::new([&SPELL, &SOURCE, &LAND, &BLINK_LAND, &WATCH])));
    let filler = table.id_of(&LAND);
    let mut game = Game::new(table, &Setup { decks: vec![vec![filler; 30], vec![filler; 30]], ..Default::default() });
    let put = |game: &mut Game, def: &'static CardDef| {
        let id = game.state.push_obj(Obj::new(table.id_of(def), YOU, Zone::BATTLEFIELD));
        game.state.zone_push(Zone::BATTLEFIELD, id);
        id
    };
    let source = put(&mut game, &SOURCE);
    let land = put(&mut game, &LAND);
    put(&mut game, &WATCH);
    let request = main(&mut game, YOU);
    assert!(matches!(request.kind, RequestKind::Priority { .. }));
    ops::become_prepared(&mut cx(&mut game), source);
    // Rebuild the offered priority after the test fixture's designation changed.
    refresh_priority(&mut game);
    let copy = game.state.zone(Zone::exile(YOU))[0];
    (game, source, land, copy)
}

fn main(game: &mut Game, player: PlayerId) -> Request {
    for _ in 0..150 {
        game.run();
        let r = game.pending().cloned().expect("a question");
        if r.player == player && game.state.turn.active == player && game.state.turn.step() == Step::PrecombatMain && matches!(r.kind, RequestKind::Priority { .. }) { return r; }
        let answer = match &r.kind {
            RequestKind::Priority { .. } => Answer::Pass,
            RequestKind::DeclareAttackers { .. } => Answer::Attack(Vec::new()),
            RequestKind::DeclareBlockers { .. } => Answer::Block(Vec::new()),
            RequestKind::DiscardToHandSize { count, hand } => Answer::Cards(hand.iter().copied().take(*count as usize).collect()),
            other => panic!("unexpected {other:?}"),
        };
        game.answer(r.id, r.player, answer).unwrap();
    }
    panic!("main phase never reached");
}

fn action(game: &mut Game, kind: ActionKind, source: ObjectId) {
    let r = game.pending().cloned().expect("a question");
    let options = match &r.kind {
        RequestKind::Priority { options } | RequestKind::PayCosts { options, .. } => options,
        other => panic!("unexpected {other:?}"),
    };
    let option = options.iter().find(|o| o.kind == kind && o.source == Some(source)).unwrap_or_else(|| panic!("{kind:?} on {source:?} not offered: {r:?}"));
    game.answer(r.id, r.player, Answer::Act(Action::of(option))).unwrap();
}

fn finish_payment(game: &mut Game, source: ObjectId, land: ObjectId) {
    assert!(game.state.obj(source).prepared);
    action(game, ActionKind::ActivateAbility, source);
    assert!(game.state.obj(source).prepared, "still prepared after first mana ability");
    action(game, ActionKind::ActivateAbility, land);
    assert!(game.state.obj(source).prepared, "still prepared when the additional discard is chosen");
    let r = game.pending().cloned().unwrap();
    let RequestKind::ChooseCards { cards, .. } = &r.kind else { panic!("discard not asked: {:?}", r.kind) };
    game.answer(r.id, r.player, Answer::Cards(vec![cards[0]])).unwrap();
}

#[test]
fn preparation_survives_all_costs_then_cast_triggers_see_it_consumed() {
    let (mut g, source, land, copy) = game();
    action(&mut g, ActionKind::CastSpell, copy);
    finish_payment(&mut g, source, land);
    assert!(!g.state.obj(source).prepared);
    assert!(g.state.zone(Zone::exile(YOU)).is_empty());
    let cast = g.history().casts().next().copied().unwrap();
    assert_eq!(cast.fact, 1, "601.2i's as-cast observation comes before consumption");
    assert_eq!(g.state.stack.iter().filter(|i| i.kind == StackKind::Triggered).count(), 1, "cast trigger saw stale preparation");
    assert_eq!(g.state.obj(copy).became, Some(cast.spell));
    let at = |matches: fn(&Event) -> bool| g.log.iter().position(|r| matches(&r.event)).unwrap();
    assert!(at(|e| matches!(e, Event::Discarded { .. })) < at(|e| matches!(e, Event::Cast(_))));
    assert!(at(|e| matches!(e, Event::Cast(_))) < at(|e| matches!(e, Event::SpellCast { .. })));
}

#[test]
fn cancelling_after_mana_payment_keeps_the_same_linked_copy_and_preparation() {
    let (mut g, source, _, copy) = game();
    action(&mut g, ActionKind::CastSpell, copy);
    action(&mut g, ActionKind::ActivateAbility, source);
    let r = g.pending().cloned().unwrap();
    g.answer(r.id, r.player, Answer::Cancel).unwrap();
    assert!(g.state.obj(source).prepared);
    assert!(!g.state.obj(source).tapped);
    assert_eq!(g.state.obj(copy).became, None);
    assert_eq!(g.state.zone(Zone::exile(YOU)), &[copy]);
    assert_eq!(g.history().casts().count(), 0);
    assert!(g.state.stack.is_empty());
    action(&mut g, ActionKind::CastSpell, copy);
}

#[test]
fn cancelling_the_additional_cost_after_mana_is_paid_keeps_preparation() {
    let (mut g, source, land, copy) = game();
    action(&mut g, ActionKind::CastSpell, copy);
    action(&mut g, ActionKind::ActivateAbility, source);
    action(&mut g, ActionKind::ActivateAbility, land);
    let r = g.pending().cloned().unwrap();
    assert!(matches!(r.kind, RequestKind::ChooseCards { .. }));
    assert!(g.state.obj(source).prepared);
    g.answer(r.id, r.player, Answer::Cancel).unwrap();
    assert_eq!(g.state.zone(Zone::exile(YOU)), &[copy]);
    assert!(g.state.obj(source).prepared && !g.state.obj(source).tapped && !g.state.obj(land).tapped);
    assert_eq!(g.history().casts().count(), 0);
}

#[test]
fn an_unpayable_prepare_copy_is_not_offered_and_keeps_its_designation() {
    let (mut g, source, land, copy) = game();
    g.state.update_obj(land, |o| o.tapped = true);
    refresh_priority(&mut g);
    let RequestKind::Priority { options } = &g.pending().unwrap().kind else { panic!() };
    assert!(!options.iter().any(|o| o.kind == ActionKind::CastSpell && o.source == Some(copy)));
    assert!(g.state.obj(source).prepared);
    assert_eq!(g.state.zone(Zone::exile(YOU)), &[copy]);
    g.state.update_obj(land, |o| o.tapped = false);
    refresh_priority(&mut g);
    action(&mut g, ActionKind::CastSpell, copy);
}

#[test]
fn current_controller_casts_and_consumes_the_original_linked_copy_despite_ability_loss() {
    let (mut g, source, land, copy) = game();
    cantrip_api::Game::gain_control(&mut cx(&mut g), source, source, THEM, Duration::Indefinite);
    cantrip_api::Game::add_effect(&mut cx(&mut g), source, source, EffectKind::LoseAllAbilities, Duration::Indefinite);
    refresh_priority(&mut g);
    main(&mut g, THEM);
    // The old exile zone still belongs to the creator, but the current permanent controller has the permission.
    assert_eq!(g.state.obj(copy).owner, YOU);
    assert!(g.state.obj(source).prepared);
    g.state.add_mana(THEM, Mana::U, 2, land, None, ManaLasts::Step);
    refresh_priority(&mut g);
    action(&mut g, ActionKind::CastSpell, copy);
    let r = g.pending().cloned().unwrap();
    let RequestKind::ChooseCards { cards, .. } = &r.kind else { panic!("{:?}", r.kind) };
    g.answer(r.id, THEM, Answer::Cards(vec![cards[0]])).unwrap();
    let cast = g.history().casts().next().copied().unwrap();
    assert_eq!(cast.player, THEM);
    assert_eq!(g.state.obj(cast.spell).controller, THEM, "CR 601.2a: caster controls spell, not linked copy's owner");
    assert!(!g.state.obj(source).prepared);
}

#[test]
fn copying_a_completed_prepare_spell_does_not_consume_another_preparation() {
    let (mut g, source, land, copy) = game();
    action(&mut g, ActionKind::CastSpell, copy);
    finish_payment(&mut g, source, land);
    let spell = g.history().casts().next().unwrap().spell;
    ops::become_prepared(&mut cx(&mut g), source);
    let new_copy = g.state.zone(Zone::exile(YOU))[0];
    ops::copy_spell(&mut cx(&mut g), spell, YOU, false).unwrap();
    assert!(g.state.obj(source).prepared);
    assert_eq!(g.state.zone(Zone::exile(YOU)), &[new_copy]);
    assert_eq!(g.history().casts().count(), 1, "a copy was not cast");
    crate::resolve::counter(&mut cx(&mut g), spell);
    assert!(g.state.obj(source).prepared, "countering an old spell consumed fresh preparation");
    assert_eq!(g.state.zone(Zone::exile(YOU)), &[new_copy]);
}

#[test]
fn a_source_leaving_while_paying_does_not_consume_a_new_objects_preparation() {
    let (mut g, source, land, copy) = game();
    g.state.update_obj(land, |o| o.card = g.cards.id_of(&BLINK_LAND));
    refresh_priority(&mut g);
    action(&mut g, ActionKind::CastSpell, copy);
    finish_payment(&mut g, source, land);
    let returned = g.state.battlefield().iter().copied().find(|&id| g.state.obj(id).card == g.cards.id_of(&SOURCE)).unwrap();
    assert_ne!(returned, source);
    assert!(g.state.obj(returned).prepared);
    let fresh = g.state.zone(Zone::exile(YOU))[0];
    assert_eq!(g.state.obj(fresh).prepared_from, Some(returned));
    assert_eq!(g.history().casts().count(), 1);
}
