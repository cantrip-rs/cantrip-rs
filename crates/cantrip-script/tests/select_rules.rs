//! The ready-made selection rules (`cantrip_mechanics::select`) judged on a real game of test-only cards, and the core
//! refusing an answer its question's rule refuses (`cantrip_api::select`, docs/engine-design.md "Selection rules"): the
//! counts, the qualities of each card, "with different names" (CR 201.2b, a split card sharing a name with a card named for
//! either half, CR 201.2a), "with different mana values" and "with total mana value N or less" (CR 202.3).

use std::sync::OnceLock;

use cantrip_api::Game as Rules;
use cantrip_api::select::{first_answer, judge};
use cantrip_engine::*;
use cantrip_mechanics::library::{find_to_hand, searched_cards};
use cantrip_mechanics::select::{self, DIFFERENT_NAMES};

const YOU: PlayerId = PlayerId(0);

/// "{0}: Search your library for up to two cards with different names, put them into your hand, then shuffle."
fn search(g: &mut dyn Rules, item: &StackItem) -> Result<(), Suspend> {
    find_to_hand(g, item.controller, item.object, select::cards(select::up_to(2)).with(DIFFERENT_NAMES), false)?;
    Ok(())
}

static SEEKER: CardDef = CardDef {
    name: "Probe Seeker",
    types: TypeSet::of(&[CardType::Artifact]),
    abilities: &[ability(cost("{0}"), search, "{0}: Search your library for up to two cards with different names, put them into your hand, then shuffle.")],
    ..CardDef::BLANK
};

static FIRE: CardDef = CardDef { name: "Probe Fire", types: TypeSet::of(&[CardType::Instant]), cost: ManaCost { generic: 2, ..ManaCost::ZERO }, ..CardDef::BLANK };
static ICE: CardDef = CardDef { name: "Probe Ice", types: TypeSet::of(&[CardType::Instant]), cost: ManaCost { generic: 2, ..ManaCost::ZERO }, ..CardDef::BLANK };
/// A split card (CR 709.4a): it has both its halves' names, "Probe Fire" and "Probe Ice".
static FIRE_ICE: CardDef = CardDef { name: "Probe Fire // Probe Ice", split: Some((&FIRE, &ICE)), ..FIRE };
static BEAR: CardDef = CardDef {
    name: "Probe Bear",
    types: TypeSet::of(&[CardType::Creature]),
    cost: ManaCost { generic: 3, ..ManaCost::ZERO },
    power: Some(2),
    toughness: Some(2),
    ..CardDef::BLANK
};
/// An artifact creature and an artifact: with the Bear (a creature) and the Wastes (a land), what "that share a card type" is tried on.
static GOLEM: CardDef = CardDef {
    name: "Probe Golem",
    types: TypeSet::of(&[CardType::Artifact, CardType::Creature]),
    cost: ManaCost { generic: 2, ..ManaCost::ZERO },
    power: Some(1),
    toughness: Some(1),
    ..CardDef::BLANK
};
static ROCK: CardDef = CardDef { name: "Probe Rock", types: TypeSet::of(&[CardType::Artifact]), cost: ManaCost { generic: 1, ..ManaCost::ZERO }, ..CardDef::BLANK };
static WASTES: CardDef = CardDef {
    name: "Probe Wastes",
    has_cost: false,
    types: TypeSet::of(&[CardType::Land]),
    supertypes: Supertypes { basic: true, legendary: false, snow: false },
    ..CardDef::BLANK
};
static FILLER: CardDef = CardDef { name: "Filler", has_cost: false, types: TypeSet::of(&[CardType::Land]), ..CardDef::BLANK };

fn table() -> &'static CardTable {
    static TABLE: OnceLock<CardTable> = OnceLock::new();
    TABLE.get_or_init(|| CardTable::new([&SEEKER, &FIRE, &ICE, &FIRE_ICE, &BEAR, &GOLEM, &ROCK, &WASTES, &FILLER]))
}

fn card(name: &str) -> CardId {
    table().find(name).unwrap_or_else(|| panic!("{name}"))
}

/// A game whose first seat's library holds, top first, `library` and then fillers, with Probe Seeker in its opening hand.
fn game(library: &[&str]) -> Game {
    let filler = card("Filler");
    let mut deck = vec![card("Probe Seeker")];
    deck.extend(std::iter::repeat_n(filler, 6));
    deck.extend(library.iter().map(|n| card(n)));
    let mut game = Game::new(table(), &Setup { decks: vec![deck, vec![filler; 20]], seed: 1, shuffle: false, opening_chooser: None, ..Default::default() });
    game.verify = true;
    game
}

/// The library's cards that are not fillers, by name, in the order a search lists them.
fn named(game: &Game) -> Vec<(ObjectId, &'static str)> {
    searched_cards(game, YOU).into_iter().map(|c| (c, game.name(c))).filter(|&(_, n)| n != "Filler").collect()
}

fn of(game: &Game, name: &str) -> ObjectId {
    named(game).into_iter().find(|&(_, n)| n == name).unwrap_or_else(|| panic!("{name} in the library")).0
}

/// What `rule` says of `selected` among the library's cards: complete, and the names of the cards that may be added.
fn verdict(game: &Game, rule: &Selection, selected: &[ObjectId]) -> (bool, Vec<&'static str>) {
    let candidates = searched_cards(game, YOU);
    let v = judge(game, rule, &Choice { chooser: YOU, source: ObjectId::NONE, candidates: &candidates, selected });
    let mut names: Vec<&'static str> = v.addable_cards(&Choice { chooser: YOU, source: ObjectId::NONE, candidates: &candidates, selected }).into_iter().map(|c| game.name(c)).filter(|&n| n != "Filler").collect();
    names.dedup();
    (v.complete, names)
}

/// "With different names" (CR 201.2b): a split card has the name of each half (CR 201.2a, 709.4a), so "Probe Fire // Probe
/// Ice" and "Probe Ice" do not have different names, nor it and "Probe Fire"; "Probe Fire" and "Probe Ice" do.
#[test]
fn different_names_keeps_a_split_card_from_a_card_named_for_either_half() {
    let game = game(&["Probe Fire // Probe Ice", "Probe Ice", "Probe Fire"]);
    let rule = select::cards(select::up_to(2)).with(DIFFERENT_NAMES);
    let (split, ice, fire) = (of(&game, "Probe Fire // Probe Ice"), of(&game, "Probe Ice"), of(&game, "Probe Fire"));
    assert_eq!(verdict(&game, &rule, &[]).0, true, "none is an answer to up to two");
    // With the split card chosen, neither half's namesake may join it; a filler may.
    let (complete, addable) = verdict(&game, &rule, &[split]);
    assert!(complete);
    assert!(!addable.contains(&"Probe Ice") && !addable.contains(&"Probe Fire"), "{addable:?}");
    assert!(verdict(&game, &rule, &[split, ice]).0 == false, "a split card and its half's namesake share a name");
    assert!(verdict(&game, &rule, &[fire, split]).0 == false);
    // Two cards whose names differ.
    let (complete, addable) = verdict(&game, &rule, &[ice]);
    assert!(complete && addable.contains(&"Probe Fire") && !addable.contains(&"Probe Fire // Probe Ice"), "{addable:?}");
    assert!(verdict(&game, &rule, &[ice, fire]).0);
}

/// The counts: exactly N, up to N, any number, and as many as can be found.
#[test]
fn the_counts_take_what_they_say() {
    let game = game(&["Probe Bear", "Probe Wastes", "Probe Fire"]);
    let (bear, wastes, fire) = (of(&game, "Probe Bear"), of(&game, "Probe Wastes"), of(&game, "Probe Fire"));
    let exactly = select::cards(select::exactly(2));
    assert!(!verdict(&game, &exactly, &[]).0 && !verdict(&game, &exactly, &[bear]).0 && verdict(&game, &exactly, &[bear, fire]).0);
    assert!(verdict(&game, &exactly, &[bear, fire]).1.is_empty(), "nothing more once two are chosen");
    let up_to = select::cards(select::up_to(1));
    assert!(verdict(&game, &up_to, &[]).0 && verdict(&game, &up_to, &[bear]).0 && verdict(&game, &up_to, &[bear]).1.is_empty());
    let any = select::cards(select::ANY_NUMBER);
    assert!(verdict(&game, &any, &[]).0 && verdict(&game, &any, &[bear, wastes, fire]).0);
    // "A basic land card" with as many as can be found: the one there is, and nothing less (CR 701.23b aside).
    let found = select::basic_land_cards(select::as_many_as_can(2));
    assert_eq!(verdict(&game, &found, &[]), (false, vec!["Probe Wastes"]));
    assert_eq!(verdict(&game, &found, &[wastes]), (true, vec![]));
    assert!(!verdict(&game, &found, &[bear]).0, "a creature is no basic land card");
}

/// The qualities of each card: a basic land card, a land card, a creature card, mana value N or less, a card named so.
#[test]
fn the_qualities_allow_only_the_cards_that_have_them() {
    let game = game(&["Probe Bear", "Probe Wastes", "Probe Fire // Probe Ice"]);
    assert_eq!(verdict(&game, &select::BASIC_LAND_CARD, &[]).1, vec!["Probe Wastes"]);
    assert_eq!(verdict(&game, &select::CREATURE_CARD, &[]).1, vec!["Probe Bear"]);
    let land = verdict(&game, &select::LAND_CARD, &[]).1;
    assert!(land.contains(&"Probe Wastes") && !land.contains(&"Probe Bear"));
    let cheap = select::cards(select::up_to(1)).with(select::mana_value_at_most(2));
    let names = verdict(&game, &cheap, &[]).1;
    assert!(!names.contains(&"Probe Bear") && names.contains(&"Probe Wastes"), "{names:?}");
    // A split card has the name of each half (CR 201.2a).
    let named = select::cards(select::up_to(1)).with(select::named(card("Probe Ice")));
    assert_eq!(verdict(&game, &named, &[]).1, vec!["Probe Fire // Probe Ice"]);
    assert_eq!(named.words(&|n| game.cards.get(CardId(n as u16)).name.to_string()), vec!["named Probe Ice".to_string()]);
}

/// "With total mana value N or less" (CR 202.3): two 2s fit in 4, not in 3, and a 3 alone fits in 3.
#[test]
fn a_total_mana_value_limits_the_cards_together() {
    let game = game(&["Probe Fire", "Probe Ice", "Probe Bear"]);
    let (fire, ice, bear) = (of(&game, "Probe Fire"), of(&game, "Probe Ice"), of(&game, "Probe Bear"));
    let four = select::cards(select::up_to(2)).with(select::total_mana_value_at_most(4));
    assert!(verdict(&game, &four, &[fire, ice]).0);
    assert!(!verdict(&game, &four, &[fire]).1.contains(&"Probe Bear"), "2 and 3 make 5");
    let three = select::cards(select::up_to(2)).with(select::total_mana_value_at_most(3));
    assert!(!verdict(&game, &three, &[fire, ice]).0);
    assert!(verdict(&game, &three, &[bear]).0);
    assert_eq!(three.words(&|n| n.to_string()), vec!["with total mana value 3 or less".to_string()]);
}

/// "With different mana values" (CR 202.3): two 2s are refused together, a 2 and a 3 taken, and once a 2 is chosen the
/// other 2 may not be added.
#[test]
fn different_mana_values_keep_the_cards_apart() {
    let game = game(&["Probe Fire", "Probe Ice", "Probe Bear"]);
    let (fire, ice, bear) = (of(&game, "Probe Fire"), of(&game, "Probe Ice"), of(&game, "Probe Bear"));
    let rule = select::cards(select::up_to(3)).with(select::DIFFERENT_MANA_VALUES);
    assert!(!verdict(&game, &rule, &[fire, ice]).0);
    assert!(verdict(&game, &rule, &[fire, bear]).0);
    let after_fire = verdict(&game, &rule, &[fire]).1;
    assert!(after_fire.contains(&"Probe Bear") && !after_fire.contains(&"Probe Ice"));
    assert_eq!(rule.words(&|n| n.to_string()), vec!["with different mana values".to_string()]);
}

/// "That share a card type" (CR 205.2a): a creature and an artifact creature share the creature type, the artifact creature and an
/// artifact the artifact type, the creature and the artifact none, and a land shares none of them; once the artifact creature is chosen
/// either may follow, once the creature is the artifact may not; three do not share one though each pair does (Bear and Rock with the Golem
/// between them); and the most it allows of the four is two.
#[test]
fn sharing_a_card_type_needs_one_type_on_every_card() {
    use cantrip_api::select::first_targets;
    let game = game(&["Probe Bear", "Probe Golem", "Probe Rock", "Probe Wastes"]);
    let (bear, golem, rock) = (of(&game, "Probe Bear"), of(&game, "Probe Golem"), of(&game, "Probe Rock"));
    let rule = select::cards(select::up_to(3)).with(select::SHARE_A_CARD_TYPE);
    assert!(verdict(&game, &rule, &[]).0 && verdict(&game, &rule, &[bear]).0, "none and one share a type");
    assert!(verdict(&game, &rule, &[bear, golem]).0 && verdict(&game, &rule, &[golem, rock]).0);
    assert!(!verdict(&game, &rule, &[bear, rock]).0, "a creature and an artifact");
    assert!(!verdict(&game, &rule, &[bear, golem, rock]).0, "each pair shares a type, the three do not");
    let addable = |selected: &[ObjectId]| {
        let mut names = verdict(&game, &rule, selected).1;
        names.sort();
        names
    };
    assert_eq!(addable(&[]), vec!["Probe Bear", "Probe Golem", "Probe Rock", "Probe Wastes"]);
    assert_eq!(addable(&[golem]), vec!["Probe Bear", "Probe Rock"]);
    assert_eq!(addable(&[bear]), vec!["Probe Golem"]);
    assert_eq!(addable(&[rock]), vec!["Probe Golem"]);
    let cards: Vec<Target> = ["Probe Bear", "Probe Golem", "Probe Rock", "Probe Wastes"].iter().map(|n| Target::Object(of(&game, n))).collect();
    let most = |rule: &Part| (0..=cards.len()).rev().find(|&n| first_targets(&game, Some(rule), YOU, ObjectId::NONE, &cards, n).is_some());
    assert_eq!(most(&select::SHARE_A_CARD_TYPE), Some(2));
    assert_eq!(select::cards(select::up_to(2)).with(select::SHARE_A_CARD_TYPE).words(&|n| n.to_string()), vec!["that share a card type".to_string()]);
}

/// How many cards a set rule allows together at most (`cantrip_api::select::first_targets`, a set rule's `LargestFn`): with
/// different names, the split card Fire // Ice shares a name with both Fire and Ice, so of the four, Fire, Ice and the Bear
/// (three, not the split card with the Bear's two); with different mana values, one of each mana value; within a
/// total of 4, the smallest first (2 and 2).
#[test]
fn a_set_rule_says_how_many_cards_it_allows_together_at_most() {
    use cantrip_api::select::first_targets;
    let game = game(&["Probe Fire // Probe Ice", "Probe Fire", "Probe Ice", "Probe Bear"]);
    let cards: Vec<Target> =
        ["Probe Fire // Probe Ice", "Probe Fire", "Probe Ice", "Probe Bear"].iter().map(|n| Target::Object(of(&game, n))).collect();
    let most = |rule: &Part| (0..=cards.len()).rev().find(|&n| first_targets(&game, Some(rule), YOU, ObjectId::NONE, &cards, n).is_some());
    assert_eq!(most(&DIFFERENT_NAMES), Some(3));
    let three = first_targets(&game, Some(&DIFFERENT_NAMES), YOU, ObjectId::NONE, &cards, 3).expect("three with different names");
    assert!(!three.contains(&cards[0]), "the split card shares a name with each half");
    assert_eq!(most(&select::DIFFERENT_MANA_VALUES), Some(2), "the test table's split card, Fire and Ice are worth 2 each, the Bear 3");
    assert_eq!(most(&select::total_mana_value_at_most(4)), Some(2));
    assert_eq!(most(&select::total_mana_value_at_most(1)), Some(0));
}

/// The first answer reached by taking the first addable card each time: the most a rule allows, or the fewest it needs.
#[test]
fn the_first_answer_takes_what_may_be_added_in_order() {
    let game = game(&["Probe Fire // Probe Ice", "Probe Ice", "Probe Fire"]);
    let library = searched_cards(&game, YOU);
    let rule = select::cards(select::exactly(2)).with(DIFFERENT_NAMES);
    let answer = first_answer(&game, &rule, YOU, ObjectId::NONE, &library, true).expect("an answer");
    assert_eq!(answer.len(), 2);
    assert!(judge(&game, &rule, &Choice { chooser: YOU, source: ObjectId::NONE, candidates: &library, selected: &answer }).complete);
    assert_eq!(first_answer(&game, &select::BASIC_LAND_CARD, YOU, ObjectId::NONE, &library, false), Some(Vec::new()), "none is an answer");
}

/// Bot1 activates Probe Seeker and is asked its search: the question carries the rule, and the core refuses the answer the
/// rule refuses - a split card with its half's namesake - and takes one it allows.
#[test]
fn the_core_refuses_an_answer_its_rule_refuses() {
    let mut game = game(&["Probe Fire // Probe Ice", "Probe Ice", "Probe Fire"]);
    let mut steps = [ActionKind::CastSpell, ActionKind::ActivateAbility].into_iter().peekable();
    for _ in 0..200 {
        game.run();
        let request = game.pending().cloned().expect("a question");
        let answer = match &request.kind {
            RequestKind::Priority { options } if request.player == YOU => {
                let option = steps.peek().and_then(|&kind| options.iter().find(|o| o.kind == kind && o.source.is_some_and(|s| game.name(s) == "Probe Seeker")));
                match option {
                    Some(option) => {
                        steps.next();
                        Answer::Act(Action::of(option))
                    }
                    None => Answer::Pass,
                }
            }
            RequestKind::Priority { .. } => Answer::Pass,
            RequestKind::SearchLibrary { rule, .. } => {
                assert_eq!(rule.count, Count::UpTo(2));
                let (split, ice, fire) = (of(&game, "Probe Fire // Probe Ice"), of(&game, "Probe Ice"), of(&game, "Probe Fire"));
                assert!(game.answer(request.id, request.player, Answer::Cards(vec![split, ice])).is_err(), "Probe Fire // Probe Ice and Probe Ice share a name");
                assert!(game.answer(request.id, request.player, Answer::Cards(vec![ice, ice])).is_err(), "the same card twice");
                game.answer(request.id, request.player, Answer::Cards(vec![ice, fire])).expect("different names");
                let hand: Vec<&str> = game.state.zone(Zone::hand(YOU)).iter().map(|&c| game.name(c)).collect();
                assert!(hand.contains(&"Probe Ice") && hand.contains(&"Probe Fire"), "{hand:?}");
                return;
            }
            other => panic!("unexpected question {other:?}"),
        };
        game.answer(request.id, request.player, answer).unwrap();
    }
    panic!("the search was never asked");
}

fn unreachable_part(_: &dyn Query, _: &Choice, _: u32) -> Verdict {
    panic!("a part was run over a selection holding a card that is not a candidate")
}

/// A card that is none of the question's candidates (a card in hand, say) is no part of any answer: the rule says so
/// without running its parts over it, which read the characteristics of the cards they are given.
#[test]
fn a_selected_card_that_is_not_a_candidate_is_refused_without_running_the_parts() {
    let game = game(&["Probe Fire", "Probe Ice"]);
    let in_hand = game.state.zone(Zone::hand(YOU))[0];
    let candidates = searched_cards(&game, YOU);
    assert!(!candidates.contains(&in_hand));
    let rule = select::cards(select::up_to(2)).with(select::part(&(unreachable_part as SelectFn), 0, ""));
    let v = judge(&game, &rule, &Choice { chooser: YOU, source: ObjectId::NONE, candidates: &candidates, selected: &[in_hand] });
    assert!(!v.complete && !v.addable.any());
    // A card the rule's list leaves out is no candidate either.
    let listed = Selection::listed(Count::UpTo(2), &candidates, vec![candidates[0]]).with(select::part(&(unreachable_part as SelectFn), 0, ""));
    let v = judge(&game, &listed, &Choice { chooser: YOU, source: ObjectId::NONE, candidates: &candidates, selected: &[candidates[1]] });
    assert!(!v.complete && !v.addable.any());
}

/// A rule holds any number of parts: five together all judge, none dropped and none a panic.
#[test]
fn a_rule_holds_more_than_four_parts() {
    let game = game(&["Probe Wastes", "Probe Bear"]);
    let wastes = of(&game, "Probe Wastes");
    let rule = select::basic_land_cards(select::up_to(2))
        .with(select::mana_value_at_most(5))
        .with(select::mana_value_at_most(4))
        .with(select::mana_value_at_most(3))
        .with(select::mana_value_at_most(2))
        .with(select::mana_value_at_most(0))
        .with(select::total_mana_value_at_most(1));
    assert_eq!(rule.parts().count(), 7);
    assert_eq!(verdict(&game, &rule, &[]), (true, vec!["Probe Wastes"]));
    assert!(verdict(&game, &rule, &[wastes]).0);
    // The fifth (and sixth) parts still judge: a fixed-cost card is refused by mana_value_at_most(0).
    let strict = select::cards(select::up_to(1)).with(select::mana_value_at_most(5)).with(select::mana_value_at_most(5)).with(select::mana_value_at_most(5)).with(select::mana_value_at_most(5)).with(select::mana_value_at_most(0));
    assert_eq!(verdict(&game, &strict, &[]).1, vec!["Probe Wastes"], "the bear's 3 passes four parts and fails the fifth");
}
