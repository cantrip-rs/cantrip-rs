//! Captured source quality and explicit {S}/ordinary allocation cross the actual host protocol.
use cantrip_engine::*;
use cantrip_host::{handles::HandleSpace, prompt::prompt, protocol::*, view::player_view};
use std::collections::HashSet;
const YOU: PlayerId = PlayerId(0);
static LAND: CardDef = CardDef { name: "Snow wire source", types: TypeSet::of(&[CardType::Land]), ..CardDef::BLANK };
fn nothing(_: &mut dyn cantrip_api::Game, _: &StackItem) -> Result<(), Suspend> { Ok(()) }
static SPELL: CardDef = CardDef { name: "Snow wire spell", cost: mana("{U}{S}"), has_cost: true, types: TypeSet::of(&[CardType::Sorcery]), spell: spell(&[], nothing), ..CardDef::BLANK };
#[test]
fn snow_pool_payment_flags_and_forged_quality_round_trip() {
    let cards = Box::leak(Box::new(CardTable::new([&LAND, &SPELL])));
    let mut g = Game::new(cards, &Setup { decks: vec![vec![cards.id_of(&LAND); 20]; 2], ..Default::default() });
    while !g.state.agenda.is_empty() { g.state.frame_remove(0); }
    g.state.update_turn(|t| { t.active = YOU; t.priority = YOU; t.set_step(Step::PrecombatMain); });
    g.state.frame_push(Frame::new(FrameKind::Priority));
    let source = g.state.zone(Zone::library(YOU))[0];
    let spell = g.state.push_obj(Obj::new(cards.id_of(&SPELL), YOU, Zone::hand(YOU)));
    g.state.zone_push(Zone::hand(YOU), spell);
    g.state.add_mana(YOU, Mana::U, 2, source, None, ManaLasts::Step, true);
    g.state.add_mana(YOU, Mana::R, 1, source, None, ManaLasts::Step, true);
    g.run();
    let request = g.pending().unwrap().clone();
    let RequestKind::Priority { options } = &request.kind else { panic!("priority") };
    let action = Action::of(options.iter().find(|o| o.kind == ActionKind::CastSpell && o.source == Some(spell)).unwrap());
    g.answer(request.id, YOU, Answer::Act(action)).unwrap(); g.run();
    let mut handles = HandleSpace::new();
    let empty = HashSet::new();
    let view = player_view(&g, YOU, &mut handles, &["one".into(), "two".into()], &[false; 2], &empty, &empty, &empty);
    let me = &view.players[0];
    assert_eq!(me.mana, [0; 6]);
    assert_eq!(me.restricted_mana.iter().filter(|m| m.snow).map(|m| m.amount).sum::<i32>(), 3);
    let request = g.pending().unwrap().clone();
    let PromptPayload::PayCostsPromptMessage(pay) = prompt(&g, &request, &mut handles).unwrap().0 else { panic!("pay") };
    assert_eq!(pay.remaining_snow, 1);
    let choices: Vec<_> = pay.options.iter().filter(|o| o.kind == ActionKindView::AssignMana && o.produces == vec![ManaTypeView::Blue]).collect();
    assert_eq!(choices.len(), 2);
    for option in choices {
        assert!(option.snow);
        assert!(option.description.contains("snow"));
        let wire = cantrip_host::protocol::TakeActionAnswer::take(option);
        let payload = AnswerPayload::TakeActionAnswer(wire.clone());
        let engine = cantrip_host::answer::translate(&payload, &request, &handles, &g).unwrap_or_else(|_| panic!("wire translates"));
        assert!(cantrip_engine::check_answer(&g, &request, &engine).is_ok());
        let mut forged = wire.clone(); forged.snow = false;
        assert!(cantrip_host::answer::translate(&AnswerPayload::TakeActionAnswer(forged), &request, &handles, &g).is_err());
        let json = serde_json::to_value(&wire).unwrap();
        assert_eq!(json["Snow"], true);
        assert_eq!(json["PaySnow"], option.pay_snow);
    }
}

#[test]
fn real_mana_undo_restores_the_captured_quality_rows() {
    static SNOW: CardDef = CardDef { name: "Undo snow land", supertypes: Supertypes { basic: false, legendary: false, snow: true, world: false }, abilities: &[mana_ability(TAP, add(Mana::U), "{T}: Add {U}.")], ..LAND };
    let cards = Box::leak(Box::new(CardTable::new([&LAND, &SNOW])));
    let mut g = Game::new(cards, &Setup { decks: vec![vec![cards.id_of(&LAND); 20]; 2], ..Default::default() });
    while !g.state.agenda.is_empty() { g.state.frame_remove(0); }
    g.state.update_turn(|t| { t.active = YOU; t.priority = YOU; t.set_step(Step::PrecombatMain); });
    g.state.frame_push(Frame::new(FrameKind::Priority));
    let source = g.state.push_obj(Obj::new(cards.id_of(&SNOW), YOU, Zone::BATTLEFIELD)); g.state.zone_push(Zone::BATTLEFIELD, source);
    // Ordinary mana from the very same source is an independent, earlier production snapshot.
    g.state.add_mana(YOU, Mana::U, 1, source, None, ManaLasts::Step, false);
    g.run(); let request = g.pending().unwrap().clone();
    let RequestKind::Priority { options } = &request.kind else { panic!("priority") };
    let action = Answer::Act(Action::of(options.iter().find(|o| o.source == Some(source)).unwrap()));
    let before = cantrip_engine::undo::Undo::<()>::before(&g, YOU, &action);
    g.answer(request.id, YOU, action).unwrap(); g.run();
    let mut undo = cantrip_engine::undo::Undo::<()>::default();
    assert!(undo.may_take_back(&g, &before)); undo.push(before, ());
    assert_eq!(g.state.mana_from_of(YOU).filter(|m| m.snow).map(|m| m.pool.total()).sum::<u32>(), 1);
    assert!(undo.take_back(&mut g, YOU).is_some());
    assert_eq!(g.state.player(YOU).pool.get(Mana::U), 1);
    assert!(g.state.mana_from_of(YOU).all(|m| !m.snow));
    assert!(!g.state.obj(source).tapped);
}

