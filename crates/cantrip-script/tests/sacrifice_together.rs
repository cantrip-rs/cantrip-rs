//! "Sacrifice N [permanents] with different names" as a cost (CR 601.2h, 602.2b, 201.2b), on a table of test-only cards: the
//! permanents are chosen as one group, by a question that carries the rule of the group (`Cost::sacrifice_together`,
//! `select::DIFFERENT_NAMES`), the core refuses an answer the rule refuses, and the ability is offered only when the
//! permanents that could pay hold a group it allows (CR 118.3). Transmutation Font's cost is the printed case.

use std::sync::OnceLock;

use cantrip_api::Game as Rules;
use cantrip_engine::*;
use cantrip_mechanics::select::DIFFERENT_NAMES;

fn artifact(q: &dyn Query, _: &Asker, id: ObjectId) -> bool {
    q.chars(id).is(CardType::Artifact)
}

fn gain_one(g: &mut dyn Rules, item: &StackItem) -> Result<(), Suspend> {
    g.gain_life(item.controller, 1);
    Ok(())
}

/// An enchantment with "Sacrifice two artifacts with different names: You gain 1 life."
static PRESS: CardDef = CardDef {
    name: "Probe Press",
    types: TypeSet::of(&[CardType::Enchantment]),
    abilities: &[ability(Cost::sacrifice_together(2, artifact, &DIFFERENT_NAMES), gain_one, "Sacrifice two artifacts with different names: You gain 1 life.")],
    ..CardDef::BLANK
};
static ROCK: CardDef = CardDef { name: "Probe Rock", types: TypeSet::of(&[CardType::Artifact]), ..CardDef::BLANK };
static GEM: CardDef = CardDef { name: "Probe Gem", types: TypeSet::of(&[CardType::Artifact]), ..CardDef::BLANK };
static FILLER: CardDef = CardDef { name: "Filler", has_cost: false, types: TypeSet::of(&[CardType::Land]), ..CardDef::BLANK };

fn table() -> &'static CardTable {
    static TABLE: OnceLock<CardTable> = OnceLock::new();
    TABLE.get_or_init(|| CardTable::new([&PRESS, &ROCK, &GEM, &FILLER]))
}

/// Bot1's turn 1 with `first` in its opening hand: casts each of `casts` as it is offered, then passes. `activate` makes it
/// activate Probe Press once its casts are done, and `answers` is what each question about the sacrifice is answered with
/// (names, the first card of each; every refused answer is recorded in the second result).
fn play(first: &[&str], casts: &[&str], activate: bool, answer: &[&str]) -> (Game, Vec<Selection>, Vec<ObjectId>) {
    let filler = table().find("Filler").unwrap();
    let mut deck: Vec<CardId> = first.iter().map(|n| table().find(n).unwrap()).collect();
    deck.extend(std::iter::repeat_n(filler, 30 - deck.len()));
    let mut game = Game::new(table(), &Setup { decks: vec![deck, vec![filler; 30]], seed: 1, shuffle: false, opening_chooser: None, ..Default::default() });
    game.verify = true;
    let mut asked = Vec::new();
    let mut refused = Vec::new();
    let mut casts = casts.iter().peekable();
    let mut activated = !activate;
    for _ in 0..200 {
        game.run();
        if casts.peek().is_none() && activated && game.state.stack.is_empty() && asked.len() == usize::from(activate) { break; }
        let Some(request) = game.pending().cloned() else { panic!("the game ended") };
        let answer = match &request.kind {
            RequestKind::Priority { options } if request.player == PlayerId(0) => {
                let cast = casts.peek().and_then(|&&n| options.iter().find(|o| o.kind == ActionKind::CastSpell && o.source.is_some_and(|s| game.name(s) == n)));
                let activation = (!activated && casts.peek().is_none() && game.state.stack.is_empty())
                    .then(|| options.iter().find(|o| o.kind == ActionKind::ActivateAbility && o.source.is_some_and(|s| game.name(s) == "Probe Press")))
                    .flatten();
                match (cast, activation) {
                    (Some(option), _) => {
                        casts.next();
                        Answer::Act(Action::of(option))
                    }
                    (None, Some(option)) => {
                        activated = true;
                        Answer::Act(Action::of(option))
                    }
                    (None, None) if casts.peek().is_none() && game.state.stack.is_empty() => break,
                    (None, None) => Answer::Pass,
                }
            }
            RequestKind::Priority { .. } => Answer::Pass,
            RequestKind::ChooseCards { purpose: Purpose::SacrificeCost, cards, rule, .. } => {
                asked.push((**rule).clone());
                // Each name in `answer`, as the next of its cards not yet taken.
                let mut taken: Vec<ObjectId> = Vec::new();
                for name in answer {
                    let card = cards.iter().copied().find(|&c| game.name(c) == *name && !taken.contains(&c)).unwrap_or_else(|| panic!("{name} among {cards:?}"));
                    taken.push(card);
                }
                if game.answer(request.id, request.player, Answer::Cards(taken.clone())).is_err() {
                    refused.extend(taken);
                    // A group the rule allows is the first of each name.
                    let mut names: Vec<&str> = Vec::new();
                    let ok: Vec<ObjectId> = cards.iter().copied().filter(|&c| { let n = game.name(c); let fresh = !names.contains(&n); names.push(n); fresh }).take(2).collect();
                    Answer::Cards(ok)
                } else {
                    continue;
                }
            }
            other => panic!("unexpected question {other:?}"),
        };
        game.answer(request.id, request.player, answer).unwrap();
    }
    (game, asked, refused)
}

fn offered(game: &mut Game) -> bool {
    game.run();
    let request = game.pending().cloned().expect("Bot1's priority");
    let RequestKind::Priority { options } = &request.kind else { panic!("a priority") };
    options.iter().any(|o| o.kind == ActionKind::ActivateAbility && o.source.is_some_and(|s| game.name(s) == "Probe Press"))
}

/// CR 118.3, 601.2h: two artifacts with one name are no group the cost allows, so the ability is not offered; a third with
/// another name makes one.
#[test]
fn the_ability_is_offered_only_when_a_group_with_different_names_can_pay() {
    let (mut same, ..) = play(&["Probe Press", "Probe Rock", "Probe Rock"], &["Probe Press", "Probe Rock", "Probe Rock"], false, &[]);
    assert!(!offered(&mut same), "two Rocks share a name");
    let (mut different, ..) = play(&["Probe Press", "Probe Rock", "Probe Rock", "Probe Gem"], &["Probe Press", "Probe Rock", "Probe Rock", "Probe Gem"], false, &[]);
    assert!(offered(&mut different), "a Rock and the Gem have different names");
}

/// The question carries the rule of the group; the core refuses the two Rocks and takes a Rock and the Gem, sacrificing both.
#[test]
fn the_core_refuses_a_group_the_rule_refuses() {
    let all = ["Probe Press", "Probe Rock", "Probe Rock", "Probe Gem"];
    let (game, asked, refused) = play(&all, &all, true, &["Probe Rock", "Probe Rock"]);
    assert_eq!(asked.len(), 1);
    assert_eq!(asked[0].count, Count::Exactly(2));
    assert_eq!(asked[0].words(&|_| String::new()), vec!["with different names".to_string()]);
    assert_eq!(refused.len(), 2, "the two Rocks were refused");
    let on = |name: &str| game.state.battlefield().iter().filter(|&&id| game.name(id) == name).count();
    assert_eq!((on("Probe Rock"), on("Probe Gem")), (1, 0), "a Rock and the Gem were sacrificed");
}

/// A group with different names is taken as chosen: the Gem and a Rock, either order.
#[test]
fn a_group_with_different_names_is_sacrificed() {
    let all = ["Probe Press", "Probe Rock", "Probe Rock", "Probe Gem"];
    let (game, asked, refused) = play(&all, &all, true, &["Probe Gem", "Probe Rock"]);
    assert_eq!(asked.len(), 1);
    assert!(refused.is_empty());
    let on = |name: &str| game.state.battlefield().iter().filter(|&&id| game.name(id) == name).count();
    assert_eq!((on("Probe Rock"), on("Probe Gem")), (1, 0));
}

/// The rule of a group holds only for a cost of one permanents part: `Cost::and` refuses to merge such a part with another
/// permanents cost, which would choose all of them as one group under a rule written for some of them.
#[test]
#[should_panic(expected = "one of them chosen under a rule of the group")]
fn a_group_under_a_rule_is_not_merged_with_another_permanents_cost() {
    let _ = Cost::sacrifice(1, artifact).and(Cost::sacrifice_together(2, artifact, &DIFFERENT_NAMES));
}

/// Nor is it one part of a cost of separately specified parts, whose payment chooses each part's permanents apart.
#[test]
#[should_panic(expected = "a permanents cost under a rule of the group is a cost of one part")]
fn a_group_under_a_rule_is_not_one_part_of_a_sequence() {
    static PARTS: [Cost; 2] = [Cost::sacrifice(1, artifact), Cost::sacrifice_together(2, artifact, &DIFFERENT_NAMES)];
    let _ = Cost::sequence(&PARTS);
}
