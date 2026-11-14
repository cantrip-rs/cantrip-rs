//! The core's check of a chosen card name against what the card asks for (CR 201.4a), over test cards: "a card name other
//! than a basic land card name" (Necromentia, Booby Trap, Desperate Research, Null Chamber) refuses a basic land's name,
//! snow or not, and allows any other land's or nonland card's.

use crate::{Game, Setup, check_answer, default_name, name_problem};
use cantrip_api::*;

static ISLAND: CardDef = CardDef {
    name: "Name Test Island",
    oracle: true,
    types: TypeSet::of(&[CardType::Land]),
    supertypes: Supertypes { basic: true, legendary: false, snow: false, world: false },
    ..CardDef::BLANK
};

static SNOW_COVERED_ISLAND: CardDef = CardDef {
    name: "Name Test Snow-Covered Island",
    oracle: true,
    types: TypeSet::of(&[CardType::Land]),
    supertypes: Supertypes { basic: true, legendary: false, snow: true, world: false },
    ..CardDef::BLANK
};

static NONBASIC_LAND: CardDef = CardDef { name: "Name Test Hallowed Fountain", oracle: true, types: TypeSet::of(&[CardType::Land]), ..CardDef::BLANK };

static NONLAND: CardDef = CardDef { name: "Name Test Lightning Bolt", oracle: true, types: TypeSet::of(&[CardType::Instant]), ..CardDef::BLANK };

static CREATURE: CardDef = CardDef { name: "Name Test Grizzly Bears", oracle: true, types: TypeSet::of(&[CardType::Creature]), ..CardDef::BLANK };

static ARTIFACT_CREATURE: CardDef =
    CardDef { name: "Name Test Ornithopter", oracle: true, types: TypeSet::of(&[CardType::Artifact, CardType::Creature]), ..CardDef::BLANK };

fn table() -> &'static CardTable {
    Box::leak(Box::new(CardTable::new([&ISLAND, &SNOW_COVERED_ISLAND, &NONBASIC_LAND, &NONLAND, &CREATURE, &ARTIFACT_CREATURE])))
}

fn allowed(table: &CardTable, kind: NameKind, card: &'static CardDef) -> bool {
    name_problem(table, kind, table.id_of(card)).is_none()
}

/// "A card name other than a basic land card name": a land with the supertype basic is a basic land (CR 205.4c), snow or
/// not, and its name is refused; a nonbasic land's name and a nonland card's are allowed (CR 201.4a).
#[test]
fn other_than_a_basic_land_card_name_refuses_only_basic_lands() {
    let table = table();
    let kind = NameKind::OTHER_THAN_BASIC_LAND;
    assert!(!allowed(table, kind, &ISLAND), "a basic land's name is allowed");
    assert!(!allowed(table, kind, &SNOW_COVERED_ISLAND), "a snow basic land's name is allowed");
    assert!(allowed(table, kind, &NONBASIC_LAND), "a nonbasic land's name is refused");
    assert!(allowed(table, kind, &NONLAND), "a nonland card's name is refused");
    assert_eq!(name_problem(table, kind, table.id_of(&ISLAND)).as_deref(), Some("not a card name other than a basic land card name"));
}

/// The restrictions printed before it keep their meaning: "a nonland card name" refuses every land's, "a land card name"
/// allows only a land's, and "a card name" anything a card has.
#[test]
fn land_and_nonland_card_names_keep_their_meaning() {
    let table = table();
    for card in [&ISLAND, &SNOW_COVERED_ISLAND, &NONBASIC_LAND] {
        assert!(!allowed(table, NameKind::NONLAND, card), "{} is a nonland card name", card.name);
        assert!(allowed(table, NameKind::LAND, card), "{} is not a land card name", card.name);
        assert!(allowed(table, NameKind::ANY, card));
    }
    assert!(allowed(table, NameKind::NONLAND, &NONLAND));
    assert!(!allowed(table, NameKind::LAND, &NONLAND));
    assert!(allowed(table, NameKind::ANY, &NONLAND));
}

/// Card types the name's card has, and card types it has none of: "a creature card name" (Infinite Obliteration), "an
/// artifact card name" (Dispossess), "a noncreature, nonland card name" (Council of the Absolute), and "a nonbasic land
/// card name" (Alpine Moon), a land card name other than a basic land card name.
#[test]
fn card_types_had_and_not_had_are_each_judged() {
    let table = table();
    let creature = NameKind { types: TypeSet::of(&[CardType::Creature]), ..NameKind::ANY };
    assert!(allowed(table, creature, &CREATURE) && allowed(table, creature, &ARTIFACT_CREATURE));
    assert!(!allowed(table, creature, &NONLAND));
    let artifact = NameKind { types: TypeSet::of(&[CardType::Artifact]), ..NameKind::ANY };
    assert!(allowed(table, artifact, &ARTIFACT_CREATURE) && !allowed(table, artifact, &CREATURE));
    let noncreature_nonland = NameKind { non_types: TypeSet::of(&[CardType::Creature, CardType::Land]), ..NameKind::ANY };
    assert!(allowed(table, noncreature_nonland, &NONLAND));
    assert!(!allowed(table, noncreature_nonland, &ARTIFACT_CREATURE) && !allowed(table, noncreature_nonland, &NONBASIC_LAND));
    let nonbasic_land = NameKind { other_than_basic_land: true, ..NameKind::LAND };
    assert!(allowed(table, nonbasic_land, &NONBASIC_LAND));
    assert!(!allowed(table, nonbasic_land, &ISLAND) && !allowed(table, nonbasic_land, &NONLAND));
}

/// The question's caption is the card's own words for what it asks.
#[test]
fn the_words_are_the_cards() {
    assert_eq!(NameKind::ANY.words(), "a card name");
    assert_eq!(NameKind::NONLAND.words(), "a nonland card name");
    assert_eq!(NameKind::LAND.words(), "a land card name");
    assert_eq!(NameKind::OTHER_THAN_BASIC_LAND.words(), "a card name other than a basic land card name");
    assert_eq!(NameKind { types: TypeSet::of(&[CardType::Artifact]), ..NameKind::ANY }.words(), "an artifact card name");
    assert_eq!(NameKind { non_types: TypeSet::of(&[CardType::Creature, CardType::Land]), ..NameKind::ANY }.words(), "a noncreature, nonland card name");
    assert_eq!(NameKind { other_than_basic_land: true, ..NameKind::LAND }.words(), "a nonbasic land card name");
}

/// A card name must be chosen (CR 201.3): no name is no answer to a name question, whatever it asks for, and a name it
/// allows is one.
#[test]
fn naming_nothing_is_refused() {
    let table = table();
    let game = Game::new(table, &Setup { decks: vec![Vec::new(), Vec::new()], ..Default::default() });
    for kind in [NameKind::ANY, NameKind::NONLAND, NameKind::LAND, NameKind::OTHER_THAN_BASIC_LAND] {
        let ask = RequestKind::ChooseName { source: ObjectId(0), looked_at: Vec::new(), suggestion: None, kind };
        let request = Request { id: 1, player: PlayerId(0), kind: ask, may_cancel: false };
        assert!(check_answer(&game, &request, &Answer::Name(None)).is_err(), "{} was answered with no name", kind.words());
        let named = default_name(table, kind, None).expect("some test card's name is allowed");
        assert!(check_answer(&game, &request, &Answer::Name(Some(named))).is_ok(), "{} refused its default name", kind.words());
    }
}

/// The name a player with none in mind takes is the suggestion when there is one, and otherwise the first name the question
/// allows, alphabetically - never a name it refuses.
#[test]
fn the_default_name_is_the_suggestion_or_the_first_allowed() {
    let table = table();
    assert_eq!(default_name(table, NameKind::ANY, Some(table.id_of(&CREATURE))), Some(table.id_of(&CREATURE)));
    assert_eq!(default_name(table, NameKind::ANY, None), Some(table.id_of(&CREATURE)), "Grizzly Bears is first of all");
    assert_eq!(default_name(table, NameKind::LAND, None), Some(table.id_of(&NONBASIC_LAND)), "Hallowed Fountain is the first land");
    assert_eq!(default_name(table, NameKind::NONLAND, None), Some(table.id_of(&CREATURE)));
    let artifact = NameKind { types: TypeSet::of(&[CardType::Artifact]), ..NameKind::ANY };
    assert_eq!(default_name(table, artifact, None), Some(table.id_of(&ARTIFACT_CREATURE)));
    let planeswalker = NameKind { types: TypeSet::of(&[CardType::Planeswalker]), ..NameKind::ANY };
    assert_eq!(default_name(table, planeswalker, None), None, "no test card is a planeswalker");
}
