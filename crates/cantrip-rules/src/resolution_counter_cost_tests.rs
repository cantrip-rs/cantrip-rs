//! Resolving counter payments use the same available-counter bound as storage-land activations (CR 118.3).
use super::*;
use crate::Game as RulesGame;
use cantrip_state::{Frame, FrameKind};
use std::sync::OnceLock;
const YOU: PlayerId = PlayerId(0);
const COUNTERS: Cost = Cost { variable: Some(VariableCost::RemoveAnyNumberOfCounters(CounterKind::Charge)), ..Cost::NONE };
const TWICE: Cost = Cost { components: &[COUNTERS,COUNTERS], ..COUNTERS };
const ASK: Question = Question::new("RemoveCounters", "remove counters?", "", "Remove them", "Decline");
static STORE: CardDef = CardDef { name: "Resolution counter store", types: TypeSet::of(&[CardType::Artifact]), ..CardDef::BLANK };
static MANA: CardDef = CardDef { name: "Resolution counter mana", types: TypeSet::of(&[CardType::Artifact]),
    abilities: &[mana_ability(TAP, add(Mana::C), "{T}: Add {C}.")], ..CardDef::BLANK };
fn position(amount: i32, answers: Vec<Answer>) -> (RulesGame, ObjectId) {
    static TABLE: OnceLock<CardTable> = OnceLock::new();
    let cards=TABLE.get_or_init(|| CardTable::new([&STORE,&MANA]));
    let mut g=RulesGame::new(cards,&crate::Setup { decks: vec![vec![],vec![]], ..Default::default() });
    while !g.state.agenda.is_empty() { g.state.frame_remove(0); }
    g.state.update_turn(|t| { t.active=YOU; t.priority=YOU; t.set_step(Step::PrecombatMain); });
    let source=g.state.push_obj(Obj::new(cards.id_of(&STORE),YOU,Zone::BATTLEFIELD));
    g.state.zone_push(Zone::BATTLEFIELD,source);
    g.state.update_obj(source,|o| o.x=9);
    let mana=g.state.push_obj(Obj::new(cards.id_of(&MANA),YOU,Zone::BATTLEFIELD));
    g.state.zone_push(Zone::BATTLEFIELD,mana);
    let mut frame=Frame::new(FrameKind::Act { player: YOU });
    frame.answers=answers.into_iter().map(|a|(Some(YOU),a)).collect();
    g.state.frame_push(frame);
    crate::counters::put(&mut cx(&mut g),source,CounterKind::Charge,amount,YOU,crate::counters::How::Effect);
    (g,source)
}
fn cx(g: &mut RulesGame) -> Cx<'_> {
    Cx { g, frame:0, cursor:0, mana_source:None, entering:None, lineage:vec![], damage:None,
        asked_at:None, doer:None, producing:None, reached:None }
}
fn action(kind: ActionKind) -> Answer { Answer::Act(Action::of(&ActionOption::simple(kind))) }
fn offered(amount: i32, chosen: u32, cost: &Cost) -> Vec<ActionOption> {
    let (mut g,source)=position(amount,vec![]);
    assert_eq!(pay_during_resolution(&mut cx(&mut g),YOU,source,source,cost,&ASK,Some(chosen),None,Paying::Other),Err(crate::Stop::Suspend));
    let RequestKind::UnlessPays { options, .. }=&g.pending().unwrap().kind else { panic!("payment question") };
    options.clone()
}
#[test]
fn counter_only_resolution_payment_does_not_open_a_mana_window() {
    let options=offered(3,0,&COUNTERS);
    assert!(options.iter().any(|o|o.kind==ActionKind::PayCost));
    assert!(!options.iter().any(|o|o.kind==ActionKind::ActivateAbility),"counter removal asks for no mana");
}
#[test]
fn resolving_counter_choice_ends_at_the_available_counters_including_fixed_costs() {
    for (chosen,cost) in [(3,COUNTERS),(1,COUNTERS.and(remove_counters(CounterKind::Charge,2))),(1,TWICE)] {
        let options=offered(3,chosen,&cost);
        assert!(!options.iter().any(|o|o.kind==ActionKind::IncreaseX),"cannot offer more counters than are available");
        assert!(options.iter().any(|o|o.kind==ActionKind::PayCost));
    }
}
#[test]
fn fixed_removals_beyond_the_counters_leave_no_payment_to_ask() {
    let (mut g,source)=position(1,vec![]);
    let cost=COUNTERS.and(remove_counters(CounterKind::Charge,2));
    assert_eq!(may_pay_x(&mut cx(&mut g),YOU,source,source,&cost,&ASK),Ok(None),"no X pays a cost of two fixed counters from one");
    assert!(g.pending().is_none());
    assert_eq!(g.state.obj(source).counters(CounterKind::Charge),1);
    let cx=cx(&mut g);
    assert_eq!(source_counter_x_limit(&cx,source,CounterKind::Charge,&cost),None,"no X, not X = 0 or a wrapped count");
    assert_eq!(source_counter_x_limit(&cx,source,CounterKind::Charge,&COUNTERS.and(remove_counters(CounterKind::Charge,1))),Some(0));
    assert_eq!(source_counter_x_cost(&cx,source,&remove_counters(CounterKind::Charge,1)),None,"no variable counter part");
}
#[test]
fn available_counter_counts_are_not_limited_by_the_mana_x_picker() {
    let options=offered(1001,1000,&COUNTERS);
    assert!(options.iter().any(|o|o.kind==ActionKind::IncreaseX),"1001 counters are actually present");
    let options=offered(i32::MAX,i32::MAX as u32,&COUNTERS);
    assert!(!options.iter().any(|o|o.kind==ActionKind::IncreaseX));
    assert!(options.iter().any(|o|o.kind==ActionKind::PayCost));
}
#[test]
fn chosen_counters_are_paid_and_returned_without_changing_the_sources_cast_x() {
    let mut answers=vec![action(ActionKind::IncreaseX);3];answers.push(action(ActionKind::PayCost));
    let (mut g,source)=position(3,answers);
    assert_eq!(may_pay_x(&mut cx(&mut g),YOU,source,source,&COUNTERS,&ASK),Ok(Some(3)));
    assert_eq!(g.state.obj(source).counters(CounterKind::Charge),0);
    assert_eq!(g.state.obj(source).x,9);
    assert!(g.log.iter().any(|r|matches!(r.event,Event::CountersRemoved { object, amount:3, .. } if object==source)));
}
#[test]
fn paying_zero_and_declining_are_distinct_and_remove_no_counters() {
    for (kind,result) in [(ActionKind::PayCost,Some(0)),(ActionKind::DeclineCost,None)] {
        let (mut g,source)=position(0,vec![action(kind)]);
        assert_eq!(may_pay_x(&mut cx(&mut g),YOU,source,source,&COUNTERS,&ASK),Ok(result));
        assert_eq!(g.state.obj(source).counters(CounterKind::Charge),0);
        assert!(!g.log.iter().any(|r|matches!(r.event,Event::CountersRemoved { .. })));
    }
}

#[test]
fn departed_and_phased_out_sources_cannot_supply_their_old_counters() {
    for phased in [false,true] {
        let (mut g,source)=position(3,vec![]);
        if phased { crate::phasing::phase_out(&mut cx(&mut g),&[source],None,false); }
        else { crate::ops::move_to(&mut cx(&mut g),source,Zone::graveyard(YOU)); }
        assert_eq!(pay_during_resolution(&mut cx(&mut g),YOU,source,source,&COUNTERS,&ASK,Some(0),None,Paying::Other),Err(crate::Stop::Suspend));
        let RequestKind::UnlessPays { options, .. }=&g.pending().unwrap().kind else { panic!("payment question") };
        assert!(!options.iter().any(|o|o.kind==ActionKind::IncreaseX),"last-known counters cannot be paid");
    }
}
/// Two fixed removals kept as separate components, with the variable removal on the whole only (`Cost::sequence(..).and`).
const SPLIT: Cost = Cost::sequence(&[remove_counters(CounterKind::Charge,1),remove_counters(CounterKind::Charge,1)]).and(COUNTERS);
#[test]
fn the_payable_limit_counts_every_fixed_component_and_refuses_an_x_above_it() {
    let (mut g,source)=position(3,vec![]);
    let c=cx(&mut g);
    assert_eq!(source_counter_x_limit(&c,source,CounterKind::Charge,&SPLIT),Some(1),"X is removed once beside the two fixed parts");
    assert!(resolution_rest_payable(&c,YOU,source,&COUNTERS,3,false));
    assert!(!resolution_rest_payable(&c,YOU,source,&COUNTERS,4,false),"four counters are not there to remove");
    let (mut g,source)=position(1,vec![]);
    let c=cx(&mut g);
    assert!(!resolution_rest_payable(&c,YOU,source,&SPLIT,0,false),"the two fixed parts take more than the one counter");
}
