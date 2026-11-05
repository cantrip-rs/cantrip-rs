use std::sync::OnceLock;
use cantrip_engine::*;

#[test]
fn eye_asks_for_exactly_one_source_of_damage() {
    static TABLE: OnceLock<CardTable> = OnceLock::new();
    let table = TABLE.get_or_init(|| CardTable::new(cantrip_cards::all()));
    let find = |name| table.find(name).unwrap();
    let mut deck = vec![find("Mox Pearl"), find("Mox Pearl"), find("Eye for an Eye")];
    deck.extend(std::iter::repeat_n(find("Island"), 20));
    let mut game = Game::new(table, &Setup { decks: vec![deck, vec![find("Island"); 24]], seed: 1, shuffle: false, opening_chooser: None, ..Default::default() });
    for _ in 0..100 {
        game.run();
        let req = game.pending().cloned().expect("choice reached before game ends");
        let answer = match &req.kind {
            RequestKind::ChooseCards { purpose, cards, rule, .. } => {
                assert_eq!(*purpose, Purpose::Choose);
                assert_eq!(rule.kind, "source of damage");
                assert!(matches!(rule.count, cantrip_api::select::Count::Exactly(1)));
                assert!(cards.iter().any(|&id| game.name(id) == "Mox Pearl"));
                return;
            }
            RequestKind::Priority { options } if req.player == PlayerId(0) => {
                let action = options.iter().find(|o| o.kind == ActionKind::CastSpell && o.source.is_some_and(|id| game.name(id) == "Mox Pearl"))
                    .or_else(|| options.iter().find(|o| o.kind == ActionKind::CastSpell && o.source.is_some_and(|id| game.name(id) == "Eye for an Eye")));
                action.map_or(Answer::Pass, |a| Answer::Act(Action::of(a)))
            }
            RequestKind::Priority { .. } => Answer::Pass,
            RequestKind::PayCosts { options, .. } => {
                let option = options.iter().find(|o| o.kind == ActionKind::ActivateAbility && o.source.is_some_and(|id| game.name(id) == "Mox Pearl"))
                    .or_else(|| options.iter().find(|o| o.kind == ActionKind::ConfirmCasting)).expect("mana or confirmation");
                Answer::Act(Action::of(option))
            }
            other => panic!("unexpected request {other:?}"),
        };
        game.answer(req.id, req.player, answer).unwrap();
    }
    panic!("no source choice reached");
}

#[test]
fn redirected_reflection_does_not_heal_prior_damage() {
    let table = Box::leak(Box::new(CardTable::new(cantrip_cards::all())));
    let text = include_str!("../../../scenarios/eye-for-an-eye-redirected-damage-keeps-prior-damage.cantrip");
    let script = cantrip_script::GameScript::parse(text, "redirected-reflection").unwrap();
    let result = cantrip_script::run_on(&script, table);
    assert!(result.is_ok(), "{}", result.err().map(|failure| failure.report).unwrap_or_default());
}
