//! Generic resolving exile payments, using battlefield and graveyard candidate shapes (CR 118.11, 118.12).
use super::*;
use crate::Game as RulesGame;
use cantrip_state::{Frame, FrameKind};
use std::sync::OnceLock;
const YOU: PlayerId = PlayerId(0);
const THEM: PlayerId = PlayerId(1);
const ASK: Question = Question::new("Exile", "exile objects?", "", "Exile", "Decline");
static BODY: CardDef = CardDef { name: "Exile test body", types: TypeSet::of(&[CardType::Creature]), power: Some(1), toughness: Some(1), ..CardDef::BLANK };
static HYBRID_A: CardDef = CardDef { name: "Artifact creature for exile", types: TypeSet::of(&[CardType::Artifact,CardType::Creature]), ..CardDef::BLANK };
static HYBRID_B: CardDef = CardDef { name: "Enchantment land for exile", types: TypeSet::of(&[CardType::Enchantment,CardType::Land]), ..CardDef::BLANK };
fn exile_proposal(_: &dyn Query, _: &Asker, p: &Proposal) -> bool {
    matches!(p, Proposal::ZoneChange(z) if z.from.is(ZoneKind::Battlefield) && z.to.is(ZoneKind::Exile))
}
fn return_instead(q: &dyn Query, _: &Asker, p: &mut Proposal) {
    if let Proposal::ZoneChange(z) = p { z.to = Zone::hand(q.obj(z.object).owner); }
}
static REPLACE: CardDef = CardDef { name: "Exile payment redirect", types: TypeSet::of(&[CardType::Enchantment]), replacements: &[replacement(ReplacementKind::ZoneChange, exile_proposal, return_instead, "return it to its owner's hand instead")], ..CardDef::BLANK };
fn battlefield(q: &dyn Query, _: &Asker) -> Vec<ObjectId> { q.state().battlefield().to_vec() }
fn graveyard(q: &dyn Query, a: &Asker) -> Vec<ObjectId> { q.state().zone(Zone::graveyard(a.controller)).to_vec() }
fn any(_: &dyn Query, _: &Asker, _: &[ObjectId]) -> bool { true }
fn four_types(q: &dyn Query, _: &Asker, ids: &[ObjectId]) -> bool {
    [CardType::Artifact, CardType::Creature, CardType::Enchantment, CardType::Land]
        .iter().all(|&kind| ids.iter().any(|&id| q.chars(id).is(kind)))
}
static BATTLE: ExileSelection = ExileSelection { candidates: battlefield, valid: any };
static GRAVE: ExileSelection = ExileSelection { candidates: graveyard, valid: four_types };
fn cx(g: &mut RulesGame) -> Cx<'_> { Cx { g, frame: 0, cursor: 0, mana_source: None, entering: None, lineage: vec![], damage: None, asked_at: None, doer: None, producing: None } }
fn position(answers: Vec<Answer>) -> (RulesGame, ObjectId, ObjectId, ObjectId) {
    static TABLE: OnceLock<CardTable> = OnceLock::new();
    let cards = TABLE.get_or_init(|| CardTable::new([&BODY,&REPLACE,&HYBRID_A,&HYBRID_B]));
    let mut g = RulesGame::new(cards, &crate::Setup { decks: vec![vec![],vec![]], ..Default::default() });
    while !g.state.agenda.is_empty() { g.state.frame_remove(0); }
    g.state.update_turn(|t| { t.active=YOU; t.priority=YOU; t.set_step(Step::PrecombatMain); });
    let put = |g: &mut RulesGame, player, zone| { let id = g.state.push_obj(Obj::new(cards.id_of(&BODY), player, zone)); g.state.zone_push(zone,id); id };
    let source = put(&mut g,YOU,Zone::BATTLEFIELD);
    let theirs = put(&mut g,THEM,Zone::BATTLEFIELD);
    let grave = put(&mut g,YOU,Zone::graveyard(YOU));
    let mut frame = Frame::new(FrameKind::Act { player: YOU });
    frame.answers = answers.into_iter().map(|a| (Some(YOU),a)).collect();
    g.state.frame_push(frame);
    (g,source,theirs,grave)
}
fn action(kind: ActionKind) -> Answer { Answer::Act(Action::of(&ActionOption::simple(kind))) }
#[test]
fn decline_and_zero_payment_are_distinct() {
    for (kind, expected) in [(ActionKind::DeclineCost,None),(ActionKind::PayCost,Some(0))] {
        let (mut g,source,_,_) = position(vec![action(kind),Answer::Cards(vec![])]);
        let cost = Cost { variable: Some(VariableCost::ExileAnyNumber(&BATTLE)), ..Cost::NONE };
        assert_eq!(may_pay_counted(&mut cx(&mut g),YOU,source,source,&cost,&ASK),Ok(expected));
    }
}
#[test]
fn opposing_permanent_is_a_valid_payment_and_receipted() {
    let (mut g,source,theirs,_) = position(vec![]);
    g.state.agenda[0].answers = vec![(Some(YOU),action(ActionKind::PayCost)),(Some(YOU),Answer::Cards(vec![theirs]))];
    let cost = Cost { variable: Some(VariableCost::ExileAnyNumber(&BATTLE)), ..Cost::NONE };
    assert_eq!(may_pay_counted(&mut cx(&mut g),YOU,source,source,&cost,&ASK),Ok(Some(1)));
    assert!(g.state.obj(theirs).became.is_some_and(|id| g.state.obj(id).zone.is(ZoneKind::Exile)));
    assert!(g.log.iter().any(|r| matches!(r.event,Event::ResolutionCostObjectPaid { object, .. } if object == theirs)));
}
#[test]
fn second_candidate_shape_requires_its_complete_set() {
    let (mut g,source,_,_) = position(vec![]);
    let cost = Cost { variable: Some(VariableCost::ExileAnyNumber(&GRAVE)), ..Cost::NONE };
    assert_eq!(may_pay_counted(&mut cx(&mut g),YOU,source,source,&cost,&ASK),Ok(None));
}

#[test]
fn redirected_payment_counts_the_paid_object_not_its_destination() {
    let (mut g,source,theirs,_) = position(vec![]);
    let redirect = g.state.push_obj(Obj::new(g.cards.id_of(&REPLACE),YOU,Zone::BATTLEFIELD));
    g.state.zone_push(Zone::BATTLEFIELD,redirect);
    g.state.agenda[0].answers = vec![(Some(YOU),action(ActionKind::PayCost)),(Some(YOU),Answer::Cards(vec![theirs]))];
    let cost = Cost { variable: Some(VariableCost::ExileAnyNumber(&BATTLE)), ..Cost::NONE };
    assert_eq!(may_pay_counted(&mut cx(&mut g),YOU,source,source,&cost,&ASK),Ok(Some(1)));
    assert!(g.state.obj(theirs).became.is_some_and(|id| g.state.obj(id).zone == Zone::hand(THEM)));
    assert!(g.log.iter().any(|r| matches!(r.event,Event::ResolutionCostObjectPaid { object, .. } if object == theirs)));
}

#[test]
fn winter_shaped_graveyard_payment_accepts_four_types_among_two_cards() {
    let (mut g,source,_,_) = position(vec![]);
    let a = g.state.push_obj(Obj::new(g.cards.id_of(&HYBRID_A), YOU, Zone::graveyard(YOU)));
    let b = g.state.push_obj(Obj::new(g.cards.id_of(&HYBRID_B), YOU, Zone::graveyard(YOU)));
    g.state.zone_push(Zone::graveyard(YOU), a); g.state.zone_push(Zone::graveyard(YOU), b);
    g.state.agenda[0].answers = vec![(Some(YOU),action(ActionKind::PayCost)),(Some(YOU),Answer::Cards(vec![a,b]))];
    let cost = Cost { variable: Some(VariableCost::ExileAnyNumber(&GRAVE)), ..Cost::NONE };
    assert_eq!(may_pay_counted(&mut cx(&mut g),YOU,source,source,&cost,&ASK),Ok(Some(2)));
    assert!(g.state.obj(a).became.is_some_and(|id| g.state.obj(id).zone.is(ZoneKind::Exile)));
    assert!(g.state.obj(b).became.is_some_and(|id| g.state.obj(id).zone.is(ZoneKind::Exile)));
}
