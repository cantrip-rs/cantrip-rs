//! "You may collect evidence X" paid as a spell or ability resolves (`Cost::collect_evidence_x`, CR 701.59a-b, 107.3f): X is
//! chosen in the payment's own question, raised only while the graveyard could pay it, and the cards are chosen as collect
//! evidence chooses them when it is paid. Collecting evidence 0 exiles nothing and is still collecting evidence.
use super::*;
use crate::Game as RulesGame;
use cantrip_state::{Frame, FrameKind};
use std::sync::OnceLock;
const YOU: PlayerId = PlayerId(0);
const EVIDENCE_X: Cost = Cost { collect_evidence_x: true, ..Cost::NONE };
const ASK: Question = Question::new("CollectEvidence", "collect evidence?", "", "Collect", "Decline");
static SOURCE: CardDef = CardDef { name: "Evidence X source", types: TypeSet::of(&[CardType::Artifact]), ..CardDef::BLANK };
const fn card_of_value(name: &'static str, generic: u32) -> CardDef {
    CardDef { name, types: TypeSet::of(&[CardType::Creature]), has_cost: true, cost: ManaCost { generic, ..ManaCost::ZERO }, power: Some(1), toughness: Some(1), ..CardDef::BLANK }
}
static ONE: CardDef = card_of_value("Evidence X one", 1);
static TWO: CardDef = card_of_value("Evidence X two", 2);
static THREE: CardDef = card_of_value("Evidence X three", 3);
/// A game with the source on the battlefield and one card of each mana value in `values` in the graveyard (1, 2 or 3), the
/// answers given to the questions in turn.
fn position(values: &[u8], answers: Vec<Answer>) -> (RulesGame, ObjectId, Vec<ObjectId>) {
    static TABLE: OnceLock<CardTable> = OnceLock::new();
    let cards = TABLE.get_or_init(|| CardTable::new([&SOURCE, &ONE, &TWO, &THREE]));
    let mut g = RulesGame::new(cards, &crate::Setup { decks: vec![vec![], vec![]], ..Default::default() });
    while !g.state.agenda.is_empty() { g.state.frame_remove(0); }
    g.state.update_turn(|t| { t.active = YOU; t.priority = YOU; t.set_step(Step::PrecombatMain); });
    let source = g.state.push_obj(Obj::new(cards.id_of(&SOURCE), YOU, Zone::BATTLEFIELD));
    g.state.zone_push(Zone::BATTLEFIELD, source);
    let mut found = Vec::new();
    for &v in values {
        let def = match v { 1 => &ONE, 2 => &TWO, _ => &THREE };
        let card = g.state.push_obj(Obj::new(cards.id_of(def), YOU, Zone::graveyard(YOU)));
        g.state.zone_push(Zone::graveyard(YOU), card);
        found.push(card);
    }
    let mut frame = Frame::new(FrameKind::Act { player: YOU });
    frame.answers = answers.into_iter().map(|a| (Some(YOU), a)).collect();
    g.state.frame_push(frame);
    (g, source, found)
}
fn cx(g: &mut RulesGame) -> Cx<'_> {
    Cx { g, frame: 0, cursor: 0, mana_source: None, entering: None, lineage: vec![], damage: None, asked_at: None, doer: None, producing: None, reached: None }
}
fn action(kind: ActionKind) -> Answer { Answer::Act(Action::of(&ActionOption::simple(kind))) }
/// The options of the question asked once X is `chosen`.
fn offered(values: &[u8], chosen: u32) -> Vec<ActionOption> {
    let (mut g, source, _) = position(values, vec![]);
    assert_eq!(pay_during_resolution(&mut cx(&mut g), YOU, source, source, &EVIDENCE_X, &ASK, Some(chosen), None, Paying::Other), Err(crate::Stop::Suspend));
    let RequestKind::UnlessPays { options, .. } = &g.pending().unwrap().kind else { panic!("payment question") };
    options.clone()
}
fn offers(options: &[ActionOption], kind: ActionKind) -> bool { options.iter().any(|o| o.kind == kind) }
/// The times `YOU` collected evidence (`Event::CollectedEvidence`, CR 701.59a).
fn collected(g: &RulesGame) -> usize {
    g.log.iter().filter(|rec| matches!(rec.event, Event::CollectedEvidence { player: YOU, .. })).count()
}

#[test]
fn x_is_raised_only_as_far_as_the_graveyard_could_pay() {
    // Cards of mana value 1, 2 and 3 total 6: X may be raised to 6, and no further (CR 701.59b).
    let five = offered(&[1, 2, 3], 5);
    assert!(offers(&five, ActionKind::PayCost) && offers(&five, ActionKind::IncreaseX));
    let six = offered(&[1, 2, 3], 6);
    assert!(offers(&six, ActionKind::PayCost) && !offers(&six, ActionKind::IncreaseX) && offers(&six, ActionKind::DecreaseX), "nothing pays 7");
    let none = offered(&[], 0);
    assert!(offers(&none, ActionKind::PayCost) && !offers(&none, ActionKind::IncreaseX), "collect evidence 0 takes nothing (CR 701.59a)");
}

#[test]
fn x_is_no_more_than_collect_evidence_counts() {
    // 258 in the graveyard, but X stops at 255.
    assert!(offers(&offered(&[3; 86], 254), ActionKind::IncreaseX));
    let most = offered(&[3; 86], 255);
    assert!(offers(&most, ActionKind::PayCost) && !offers(&most, ActionKind::IncreaseX));
}

#[test]
fn collect_evidence_x_exiles_cards_with_that_total_or_more_and_is_collecting_evidence() {
    // X = 4: the cards of mana value 3 and 2 total 5, which is 4 or greater (CR 701.59a).
    let (mut g, source, cards) = position(&[1, 2, 3], vec![]);
    let mut answers = vec![action(ActionKind::IncreaseX); 4];
    answers.extend([action(ActionKind::PayCost), Answer::Cards(vec![cards[1], cards[2]])]);
    g.state.frame_remove(0);
    let mut frame = Frame::new(FrameKind::Act { player: YOU });
    frame.answers = answers.into_iter().map(|a| (Some(YOU), a)).collect();
    g.state.frame_push(frame);
    assert_eq!(may_pay_x(&mut cx(&mut g), YOU, source, source, &EVIDENCE_X, &ASK), Ok(Some(4)));
    assert!(g.state.obj(cards[1]).became.is_some() && g.state.obj(cards[2]).became.is_some(), "the two chosen are exiled");
    assert!(g.state.obj(cards[0]).became.is_none(), "the rest stay");
    assert_eq!(collected(&g), 1, "it is collecting evidence");
}

#[test]
fn the_cards_are_chosen_as_collect_evidence_of_the_x_chosen() {
    let (mut g, source, cards) = position(&[1, 2, 3], vec![action(ActionKind::IncreaseX), action(ActionKind::IncreaseX), action(ActionKind::PayCost)]);
    assert_eq!(may_pay_x(&mut cx(&mut g), YOU, source, source, &EVIDENCE_X, &ASK), Err(crate::Stop::Suspend));
    let RequestKind::CollectEvidence { cards: offered, minimum, .. } = &g.pending().unwrap().kind else { panic!("collect evidence asks its cards") };
    assert_eq!((offered.clone(), *minimum), (cards, 2));
}

#[test]
fn collect_evidence_zero_exiles_nothing_and_is_still_collecting_evidence() {
    let (mut g, source, cards) = position(&[1, 2], vec![action(ActionKind::PayCost)]);
    assert_eq!(may_pay_x(&mut cx(&mut g), YOU, source, source, &EVIDENCE_X, &ASK), Ok(Some(0)));
    assert!(cards.iter().all(|&c| g.state.obj(c).became.is_none()));
    assert_eq!(collected(&g), 1);
}

#[test]
fn collect_evidence_x_declined_exiles_nothing() {
    let (mut g, source, cards) = position(&[1, 2, 3], vec![action(ActionKind::DeclineCost)]);
    assert_eq!(may_pay_x(&mut cx(&mut g), YOU, source, source, &EVIDENCE_X, &ASK), Ok(None));
    assert!(cards.iter().all(|&c| g.state.obj(c).became.is_none()));
    assert_eq!(collected(&g), 0);
}
