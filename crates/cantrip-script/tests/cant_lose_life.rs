//! A player who can't lose life (CR 119.8), on a table of test-only cards: an effect's loss does not happen, and a cost that
//! pays life is not offered. The printed case is Platinum Emperion
//! (`scenarios/platinum-emperion-leaves-its-controller-no-life-to-pay-for-mana-confluence.cantrip`).

use std::sync::OnceLock;

use cantrip_api::Game as Rules;
use cantrip_engine::*;

fn nothing(_: &mut dyn Rules, _: &StackItem) -> Result<(), Suspend> {
    Ok(())
}

fn you(_: &dyn Query, asker: &Asker, player: PlayerId) -> bool {
    player == asker.controller
}

/// "You can't lose life."
static STASIS: CardDef = CardDef {
    name: "Probe Stasis",
    types: TypeSet::of(&[CardType::Artifact]),
    cant_lose_life: &[you],
    ..CardDef::BLANK
};

fn lose_two(g: &mut dyn Rules, item: &StackItem) -> Result<(), Suspend> {
    g.lose_life(item.controller, 2);
    Ok(())
}

/// "{T}, Pay 1 life: You lose 2 life."
static DRAIN: CardDef = CardDef {
    name: "Probe Drain",
    types: TypeSet::of(&[CardType::Artifact]),
    abilities: &[
        ability(cost("{T}").and(Cost { life: 1, ..Cost::NONE }), nothing, "{T}, Pay 1 life: Nothing happens."),
        ability(cost("{T}"), lose_two, "{T}: You lose 2 life."),
    ],
    ..CardDef::BLANK
};

static FILLER: CardDef = CardDef { name: "Filler", has_cost: false, types: TypeSet::of(&[CardType::Land]), ..CardDef::BLANK };

fn table() -> &'static CardTable {
    static TABLE: OnceLock<CardTable> = OnceLock::new();
    TABLE.get_or_init(|| CardTable::new([&STASIS, &DRAIN, &FILLER]))
}

/// Plays Bot1's turn 1: each (kind, name, index of the cast or ability) in order, as it is offered; everything else passes.
/// The game.
fn play(first: &[&str], steps: &[(ActionKind, &str, u8)]) -> Game {
    let filler = table().find("Filler").unwrap();
    let mut deck: Vec<CardId> = first.iter().map(|n| table().find(n).unwrap()).collect();
    deck.extend(std::iter::repeat(filler).take(30 - deck.len()));
    let others = vec![filler; 30];
    let mut game = Game::new(table(), &Setup { decks: vec![deck, others], seed: 1, shuffle: false, opening_chooser: None, ..Default::default() });
    game.verify = true;
    let mut todo = steps.iter().peekable();
    for _ in 0..200 {
        game.run();
        if todo.peek().is_none() && game.state.stack.is_empty() && game.state.triggers.is_empty() {
            break;
        }
        let Some(request) = game.pending().cloned() else { panic!("the game ended") };
        let answer = match &request.kind {
            RequestKind::Priority { options } if request.player == PlayerId(0) => {
                let option = todo.peek().and_then(|&&(kind, name, index)| {
                    options.iter().find(|o| o.kind == kind && o.index == index && o.source.is_some_and(|s| game.name(s) == name))
                });
                match option {
                    Some(option) => {
                        todo.next();
                        Answer::Act(Action::of(option))
                    }
                    None => Answer::Pass,
                }
            }
            RequestKind::Priority { .. } => Answer::Pass,
            RequestKind::ChooseCostPayment { choices, .. } => {
                assert!(choices.iter().all(|choice| matches!(choice.kind, CostPaymentKind::Tap | CostPaymentKind::Life)));
                Answer::CostPayment(choices.iter().position(|choice| choice.kind == CostPaymentKind::Tap).unwrap_or(0))
            }
            other => panic!("unexpected question {other:?}"),
        };
        game.answer(request.id, request.player, answer).unwrap();
    }
    assert!(todo.peek().is_none(), "every step was offered");
    game
}

/// CR 119.8: a player who can't lose life loses none to an effect, and can't pay life: the ability that costs 1 life is not
/// offered (CR 602.2b, 118.3), while the one that doesn't is.
#[test]
fn a_player_who_cant_lose_life_loses_none_and_pays_none() {
    let game = play(
        &["Probe Stasis", "Probe Drain"],
        &[(ActionKind::CastSpell, "Probe Stasis", 0), (ActionKind::CastSpell, "Probe Drain", 0), (ActionKind::ActivateAbility, "Probe Drain", 1)],
    );
    assert_eq!(game.state.player(PlayerId(0)).life, 20);
}

/// The positive control: without the static ability, the payment is offered and the loss happens.
#[test]
fn a_player_who_can_lose_life_pays_and_loses() {
    let game = play(
        &["Probe Drain"],
        &[(ActionKind::CastSpell, "Probe Drain", 0), (ActionKind::ActivateAbility, "Probe Drain", 0)],
    );
    assert_eq!(game.state.player(PlayerId(0)).life, 19);
}

/// CR 119.8: with the static ability, the ability that pays life is never offered.
#[test]
fn a_player_who_cant_lose_life_is_not_offered_a_life_payment() {
    let drain_abilities = |first: &[&str], steps: &[(ActionKind, &str, u8)]| -> Vec<u8> {
        let mut game = play(first, steps);
        game.run();
        let request = game.pending().cloned().expect("Bot1's priority");
        assert_eq!(request.player, PlayerId(0));
        let RequestKind::Priority { options } = &request.kind else { panic!("a priority") };
        options
            .iter()
            .filter(|o| o.kind == ActionKind::ActivateAbility && o.source.is_some_and(|s| game.name(s) == "Probe Drain"))
            .map(|o| o.index)
            .collect()
    };
    assert_eq!(drain_abilities(&["Probe Drain"], &[(ActionKind::CastSpell, "Probe Drain", 0)]), vec![0, 1]);
    assert_eq!(
        drain_abilities(&["Probe Stasis", "Probe Drain"], &[(ActionKind::CastSpell, "Probe Stasis", 0), (ActionKind::CastSpell, "Probe Drain", 0)]),
        vec![1]
    );
}
