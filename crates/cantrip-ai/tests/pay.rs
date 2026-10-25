//! What the AI player pays with (`cantrip_ai::pay::pay`): the mana abilities it taps for a cost, from a scenario's
//! position. A land whose abilities name no mana of their own (each of Plateau's two "{T}: Add {R}" and "{T}: Add {W}")
//! is judged by what its ability adds, and the colored symbols owed are matched to sources first, so that a dual land is
//! not tapped for the color the basic land beside it could pay.

use cantrip_ai::pay;
use cantrip_engine::*;
use cantrip_script::GameScript;

const SCRIPT: &str = "\
[Bot1 Deck]
Plateau
Mountain
Hardened Academic
Mountain
Mountain
Mountain
Mountain
Mountain
Mountain
Mountain

[Bot2 Deck]
Island x10

[Actions]
Bot1.Hand.Plateau.Play
[T3 Bot1.Main1]
Bot1.Hand.Mountain.Play
";

/// Pays for the spell `pay` is asked to, answer by answer: whether the spell got onto the stack.
fn cast_and_pay(game: &mut Game, spell: &str) -> bool {
    let request = game.pending().cloned().expect("a request");
    let RequestKind::Priority { options } = &request.kind else {
        panic!("not priority: {request:?}")
    };
    let cast = options
        .iter()
        .find(|o| {
            o.kind == ActionKind::CastSpell && o.source.is_some_and(|s| game.name(s) == spell)
        })
        .expect("the spell is offered");
    game.answer(request.id, request.player, Answer::Act(Action::of(cast)))
        .expect("the cast is taken");
    for _ in 0..20 {
        let Some(request) = game.pending().cloned() else {
            return false;
        };
        match &request.kind {
            RequestKind::PayCosts { .. } => {
                let answer = pay::pay(game, &request, None).expect("pay answers the payment");
                if let Answer::Act(a) = &answer {
                    assert_ne!(
                        a.kind,
                        ActionKind::CancelCasting,
                        "the payment was cancelled"
                    );
                }
                game.answer(request.id, request.player, answer)
                    .expect("the payment is taken");
            }
            _ => {
                return game
                    .state
                    .stack
                    .iter()
                    .any(|item| game.name(item.object) == spell);
            }
        }
    }
    false
}

#[test]
fn a_dual_land_and_a_basic_pay_both_colors() {
    let script = GameScript::parse(SCRIPT, "pay-plateau").unwrap();
    let mut game = cantrip_script::run(&script)
        .unwrap_or_else(|f| panic!("{f:?}"))
        .game;
    assert!(
        cast_and_pay(&mut game, "Hardened Academic"),
        "Hardened Academic was not paid for with Plateau and a Mountain"
    );
}
