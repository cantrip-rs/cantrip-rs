//! "You may pay {X}" and "you may pay X life" as a spell or ability resolves (CR 107.3f, 118.12, 119.4): X is chosen in the
//! payment's own question and raised as far as the payer could pay it - X life no further than the life there is, X mana
//! no further than an X announced for a cast (`most_x`) - and no fixed number short of that. A scenario would have to make
//! more than a thousand mana by game actions (Gaea's Cradle under an army of thousands of Goblins) and then raise X one step
//! at a time over every one of those permanents, minutes of the corpus for one bound; and no card yet pays X life as it
//! resolves. The payment is driven here directly, as `collect_evidence_x_tests` drives its own.
use super::*;
use crate::Game as RulesGame;
use cantrip_state::{Frame, FrameKind};
use std::sync::OnceLock;
const YOU: PlayerId = PlayerId(0);
const X_MANA: Cost = Cost { mana: ManaCost { x: 1, ..ManaCost::ZERO }, ..Cost::NONE };
const X_LIFE: Cost = Cost { pay_x_life: true, ..Cost::NONE };
const ASK: Question = Question::new("Pay", "pay X?", "", "Pay", "Decline");
static SOURCE: CardDef = CardDef { name: "Resolution X source", types: TypeSet::of(&[CardType::Artifact]), ..CardDef::BLANK };

/// A game with the source on the battlefield, `YOU` at `life` with `mana` colorless mana in their pool, the answers given to
/// the questions in turn.
fn position(life: i32, mana: u32, answers: Vec<Answer>) -> (RulesGame, ObjectId) {
    static TABLE: OnceLock<CardTable> = OnceLock::new();
    let cards = TABLE.get_or_init(|| CardTable::new([&SOURCE]));
    let mut g = RulesGame::new(cards, &crate::Setup { decks: vec![vec![], vec![]], ..Default::default() });
    while !g.state.agenda.is_empty() { g.state.frame_remove(0); }
    g.state.update_turn(|t| { t.active = YOU; t.priority = YOU; t.set_step(Step::PrecombatMain); });
    let source = g.state.push_obj(Obj::new(cards.id_of(&SOURCE), YOU, Zone::BATTLEFIELD));
    g.state.zone_push(Zone::BATTLEFIELD, source);
    g.state.update_player(YOU, |p| p.life = life);
    if mana > 0 {
        g.state.add_mana(YOU, Mana::C, mana, source, None, ManaLasts::Step, false);
    }
    let mut frame = Frame::new(FrameKind::Act { player: YOU });
    frame.answers = answers.into_iter().map(|a| (Some(YOU), a)).collect();
    g.state.frame_push(frame);
    (g, source)
}
fn cx(g: &mut RulesGame) -> Cx<'_> {
    Cx { g, frame: 0, cursor: 0, mana_source: None, entering: None, lineage: vec![], damage: None, asked_at: None, doer: None, producing: None, reached: None }
}
fn action(kind: ActionKind) -> Answer { Answer::Act(Action::of(&ActionOption::simple(kind))) }
/// The options of the question asked of `cost` once X is `chosen`.
fn offered(life: i32, mana: u32, cost: &Cost, chosen: u32) -> Vec<ActionOption> {
    let (mut g, source) = position(life, mana, vec![]);
    assert_eq!(pay_during_resolution(&mut cx(&mut g), YOU, source, source, cost, &ASK, Some(chosen), None, Paying::Other), Err(crate::Stop::Suspend));
    let RequestKind::UnlessPays { options, .. } = &g.pending().unwrap().kind else { panic!("payment question") };
    options.clone()
}
fn offers(options: &[ActionOption], kind: ActionKind) -> bool { options.iter().any(|o| o.kind == kind) }
/// Raises X to `x` one step at a time, then pays.
fn raised_and_paid(x: usize) -> Vec<Answer> {
    let mut answers = vec![action(ActionKind::IncreaseX); x];
    answers.push(action(ActionKind::PayCost));
    answers
}

#[test]
fn an_x_paid_in_mana_is_raised_past_a_thousand_and_paid() {
    // 1500 mana in the pool: X goes past 1000 (CR 107.3f), and paying X = 1200 spends 1200 of it.
    let thousand = offered(20, 1500, &X_MANA, 1000);
    assert!(offers(&thousand, ActionKind::PayCost) && offers(&thousand, ActionKind::IncreaseX), "X = 1001 can be chosen");
    let (mut g, source) = position(20, 1500, raised_and_paid(1200));
    assert_eq!(may_pay_x(&mut cx(&mut g), YOU, source, source, &X_MANA, &ASK), Ok(Some(1200)));
    assert_eq!(g.state.player(YOU).pool.total(), 300);
}

#[test]
fn an_x_paid_in_life_is_raised_as_far_as_the_life_there_is() {
    // At 1500 life: X life goes past 1000, and as far as 1500, no further (CR 119.4).
    let thousand = offered(1500, 0, &X_LIFE, 1000);
    assert!(offers(&thousand, ActionKind::PayCost) && offers(&thousand, ActionKind::IncreaseX), "X = 1001 can be chosen");
    let all = offered(1500, 0, &X_LIFE, 1500);
    assert!(offers(&all, ActionKind::PayCost) && !offers(&all, ActionKind::IncreaseX), "1501 life can't be paid");
    let (mut g, source) = position(1500, 0, raised_and_paid(1200));
    assert_eq!(may_pay_x(&mut cx(&mut g), YOU, source, source, &X_LIFE, &ASK), Ok(Some(1200)));
    assert_eq!(g.state.player(YOU).life, 300);
}
