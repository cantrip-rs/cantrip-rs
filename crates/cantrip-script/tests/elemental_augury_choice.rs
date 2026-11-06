//! Elemental Augury identifies each ordering choice to clients as the lowest remaining card.
use cantrip_engine::{Answer, Count, Purpose, RequestKind};
use cantrip_script::{run, GameScript};

#[test]
fn ordering_choices_describe_the_lowest_remaining_card() {
    let scenario = include_str!("../../../scenarios/elemental-augury-reorders-opponents-library.cantrip");
    let before_choice = scenario.split("Bot1.Choose(Lightning Bolt)").next().unwrap();
    let text = format!("{before_choice}[Handover Bot1.Choose]\n");
    let script = GameScript::parse(&text, "elemental-augury-choice").unwrap();
    let result = run(&script).map_err(|failure| failure.report).unwrap();
    assert!(result.handed_over);
    let mut game = result.game;
    for expected_count in [3, 2] {
        let request = game.pending().cloned().expect("ordering choice");
        let RequestKind::ChooseCards { purpose, cards, rule, .. } = &request.kind else {
            panic!("expected a card choice: {request:?}");
        };
        assert_eq!(*purpose, Purpose::Choose);
        assert_eq!(rule.kind, "card to place lowest among the remaining cards");
        assert!(matches!(rule.count, Count::Exactly(1)));
        assert_eq!(cards.len(), expected_count);
        game.answer(request.id, request.player, Answer::Cards(vec![cards[0]])).unwrap();
    }
}
