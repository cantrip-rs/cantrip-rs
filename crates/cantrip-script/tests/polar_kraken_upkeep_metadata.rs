//! Polar Kraken's upkeep exposes its land cost and requires the entire two-land payment.
use cantrip_engine::*;
use cantrip_script::{GameScript, run};

#[test]
fn upkeep_names_its_cost_and_requires_two_lands() {
    let source = include_str!("../../../scenarios/polar-kraken-enters-tapped-tramples-and-pays-growing-upkeep.cantrip");
    let before_choice = source.split("! Bot1.Choose(Island; Island; Island)").next().unwrap();
    let script = GameScript::parse(&format!("{before_choice}[Handover Bot1.Choose]\n"), "polar-kraken-payment").unwrap();
    let result = run(&script).unwrap_or_else(|f| panic!("{}", f.report));
    let request = result.game.pending().expect("land payment");
    let RequestKind::ChooseCards { purpose, rule, cards, source, .. } = &request.kind else { panic!("expected sacrifice choice") };
    assert_eq!(*purpose, Purpose::SacrificeCost);
    assert_eq!(rule.count, Count::Exactly(2));
    assert!(cards.iter().all(|id| result.game.cards.get(result.game.state.obj(*id).card).name == "Island"));
    let card = result.game.cards.get(result.game.state.obj(*source).card);
    assert!(card.abilities.iter().any(|ability| ability.text().contains("Cumulative upkeep—Sacrifice a land.")));
}
