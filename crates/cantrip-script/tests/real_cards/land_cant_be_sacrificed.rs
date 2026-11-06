//! Tenured Tethermage's "you may sacrifice a land. If you do, create two tapped Heartwood tokens" with a land that can't be
//! sacrificed (CR 101.2, 701.21a): it is no choice - "you can't try to sacrifice [it], fail" (the rulings of every card that
//! says so) - so with no other land nothing is asked and no Heartwood is made, and beside a Forest only the Forest is offered.
//! No printed card in the engine makes a land that can't be sacrificed (Hithlain Rope is an artifact), so a test land does.

use std::sync::OnceLock;

use cantrip_engine::*;

/// "This land can't be sacrificed."
static MOOR: CardDef = CardDef {
    name: "Probe Moor",
    has_cost: false,
    types: TypeSet::of(&[CardType::Land]),
    keywords: Keywords::of(&[Keyword::CantBeSacrificed]),
    ..CardDef::BLANK
};

fn table() -> &'static CardTable {
    static TABLE: OnceLock<CardTable> = OnceLock::new();
    TABLE.get_or_init(|| CardTable::new(cantrip_cards::all().into_iter().chain([&MOOR])))
}

/// Bot1 plays `lands` (one each turn, from turn 1), casts three Moxen on turn 1 and Tenured Tethermage with them once every
/// land is played; every choice of a land to sacrifice takes the first offered. The game, and each such choice's choosable
/// lands by name.
fn play(lands: &[&str]) -> (Game, Vec<Vec<String>>) {
    let find = |n: &str| table().find(n).unwrap_or_else(|| panic!("no card {n}"));
    let mut hand: Vec<CardId> = ["Mox Ruby", "Mox Emerald", "Mox Jet", "Tenured Tethermage"].iter().map(|n| find(n)).collect();
    hand.extend(lands.iter().map(|n| find(n)));
    hand.extend(std::iter::repeat(find("Island")).take(30 - hand.len()));
    let mut game = Game::new(table(), &Setup { decks: vec![hand, vec![find("Island"); 30]], seed: 1, shuffle: false, opening_chooser: None, ..Default::default() });
    game.verify = true;
    let mut lands = lands.iter().peekable();
    let mut casts = ["Mox Ruby", "Mox Emerald", "Mox Jet", "Tenured Tethermage"].into_iter().peekable();
    let mut asked = Vec::new();
    for _ in 0..2000 {
        game.run();
        if lands.peek().is_none() && casts.peek().is_none() && game.state.stack.is_empty() && game.state.triggers.is_empty() {
            return (game, asked);
        }
        let request = game.pending().cloned().expect("the game goes on");
        let answer = match &request.kind {
            RequestKind::Priority { options } if request.player == PlayerId(0) && game.state.stack.is_empty() => {
                let named = |o: &&ActionOption, kind: ActionKind, name: &str| o.kind == kind && o.source.is_some_and(|s| game.name(s) == name);
                let land = lands.peek().and_then(|&&n| options.iter().find(|o| named(o, ActionKind::PlayLand, n)));
                // The Tethermage waits for the last land.
                let cast = casts
                    .peek()
                    .filter(|&&n| n != "Tenured Tethermage" || lands.peek().is_none())
                    .and_then(|&n| options.iter().find(|o| named(o, ActionKind::CastSpell, n)));
                match (land, cast) {
                    (Some(o), _) => {
                        lands.next();
                        Answer::Act(Action::of(o))
                    }
                    (None, Some(o)) => {
                        casts.next();
                        Answer::Act(Action::of(o))
                    }
                    _ => Answer::Pass,
                }
            }
            RequestKind::Priority { .. } => Answer::Pass,
            RequestKind::ChooseCostPayment { choices, .. } => Answer::CostPayment(choices.iter().position(|c| c.kind == CostPaymentKind::Mana).unwrap_or(0)),
            RequestKind::PayCosts { options, remaining, mana_abilities, .. } => {
                let mox = options.iter().find(|o| o.kind == ActionKind::ActivateAbility && o.source.is_some_and(|s| game.name(s).starts_with("Mox")));
                super::pay(options, remaining, *mana_abilities, mox)
            }
            RequestKind::ChooseCards { cards, rule, purpose: Purpose::Sacrifice, .. } => {
                let choosable: Vec<ObjectId> = cards.iter().copied().filter(|&c| rule.listed_card(c) && game.chars(c).is(CardType::Land)).collect();
                asked.push(choosable.iter().map(|&id| game.name(id).to_string()).collect());
                Answer::Cards(choosable.into_iter().take(1).collect())
            }
            RequestKind::DiscardToHandSize { count, hand } => Answer::Cards(hand.iter().copied().take(*count as usize).collect()),
            RequestKind::DeclareAttackers { .. } => Answer::Attack(Vec::new()),
            RequestKind::DeclareBlockers { .. } => Answer::Block(Vec::new()),
            other => panic!("unexpected question {other:?}"),
        };
        game.answer(request.id, request.player, answer).unwrap();
    }
    panic!("the game went on too long");
}

fn heartwoods(game: &Game) -> usize {
    game.state.battlefield().iter().filter(|&&id| game.name(id) == "Heartwood Token").count()
}

/// With only a land that can't be sacrificed, nothing is asked and nothing is made: the Moor stays.
#[test]
fn a_land_that_cant_be_sacrificed_is_no_choice_and_makes_no_heartwoods() {
    let (game, asked) = play(&["Probe Moor"]);
    assert!(asked.is_empty(), "asked {asked:?}");
    assert_eq!(heartwoods(&game), 0);
    assert!(game.state.battlefield().iter().any(|&id| game.name(id) == "Probe Moor"));
}

/// Beside a Forest, only the Forest is offered; sacrificed, it makes the two Heartwoods (the positive control).
#[test]
fn beside_a_forest_only_the_forest_is_offered() {
    let (game, asked) = play(&["Probe Moor", "Forest"]);
    assert_eq!(asked, vec![vec!["Forest".to_string()]]);
    let names: Vec<&str> = game.state.battlefield().iter().map(|&id| game.name(id)).collect();
    assert_eq!(heartwoods(&game), 2, "{names:?}");
    assert!(game.state.battlefield().iter().any(|&id| game.name(id) == "Probe Moor"));
}
