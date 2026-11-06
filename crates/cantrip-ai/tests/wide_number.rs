//! Exact large numeric choices through the real network and search entry points.
use std::sync::atomic::{AtomicUsize, Ordering};
use cantrip_api::Game as Rules;
use cantrip_engine::*;
use cantrip_ai::{CardIndex, Knowledge, Prior, cancel::CancelGuard, eval::{EvalError, Evaluator, FeatureBatch, Outputs}, net::{self, Choice}, search::{self, Config, WallClock}};
use cantrip_obs::{InfoMode, factor::FactoredAnswer};

fn resolve(g: &mut dyn Rules, s: &StackItem) -> Result<(), Suspend> {
    let _ = cantrip_mechanics::choose::number(g, s.controller, s.object, 0, u32::MAX, "Choose a number")?;
    Ok(())
}
static SOURCE: CardDef = CardDef { name: "Number probe", oracle: true, types: TypeSet::of(&[CardType::Sorcery]), spell: spell(&[], resolve), ..CardDef::BLANK };
static FILLER: CardDef = CardDef { name: "Filler", oracle: true, types: TypeSet::of(&[CardType::Land]), ..CardDef::BLANK };
struct LastDigit { calls: AtomicUsize }
impl Evaluator for LastDigit {
    fn evaluate(&self, b: &FeatureBatch) -> Result<Outputs, EvalError> {
        self.calls.fetch_add(1, Ordering::Relaxed);
        assert!(b.options.iter().all(|&(_, n)| n <= 16), "no range enumeration");
        Ok(Outputs { value: vec![0.0; b.rows],
            policy: b.options.iter().map(|&(_, n)| (0..n).map(|i| i as f32 * 100.0).collect()).collect(),
            aux: vec![[0.0; 2]; b.rows] })
    }
}
#[test]
fn wide_number_network_and_search_offer_the_maximum_without_enumerating_the_range() {
    let table = Box::leak(Box::new(CardTable::new([&SOURCE, &FILLER])));
    let script = cantrip_script::GameScript::parse("[Bot1 Deck]\nNumber probe\nFiller x19\n[Bot2 Deck]\nFiller x20\n[Actions]\nBot1.Hand.Number probe.Cast\n[Handover Bot1.Number]\n", "wide-number-ai").unwrap();
    let result = cantrip_script::run_on(&script, table).unwrap_or_else(|f| panic!("{}", f.report));
    assert!(result.handed_over);
    let game = result.game; let request = game.pending().unwrap();
    assert!(matches!(request.kind, RequestKind::ChooseNumber { most: u32::MAX, .. }));
    let knowledge = Knowledge::of(&game, request.player); let index = CardIndex::of(table);
    let evaluator = LastDigit { calls: AtomicUsize::new(0) };
    let answer = net::answer(&game, request, &knowledge, InfoMode::Full, &index, &evaluator, Choice::Best).unwrap();
    assert_eq!(answer.answer, Answer::Announce(u32::MAX));
    assert_eq!(answer.factored, FactoredAnswer::Number(u32::MAX));
    assert_eq!(answer.steps, 8); assert_eq!(evaluator.calls.load(Ordering::Relaxed), 8);
    let searched = search::search(&game, request, &knowledge, InfoMode::Full, &index, &evaluator,
        &Config { simulations: Some(16), batch: 1, choice: Choice::Best, ..Default::default() },
        &Prior::new(), &CancelGuard::default(), &WallClock).unwrap();
    assert_eq!(searched.answer, Answer::Announce(u32::MAX));
    let report = searched.search.unwrap(); assert_eq!(report.simulations, 16);
    assert!(report.root_edges <= 16); assert_eq!(report.refused, 0);
}
