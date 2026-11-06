//! A token's own faces (CR 707.8a), over test cards: a token made as a copy of a double-faced permanent is a double-faced
//! token, entering with the copied face up, each face under the copy's exceptions, and it transforms; a token made as a copy
//! of one face alone is single-faced and does not. The scenarios show the same with Three Steps Ahead and Ardyn, the Usurper.

use crate::ops;
use crate::{Cx, Game, Setup};
use cantrip_api::*;

const YOU: PlayerId = PlayerId(0);

static BEAST: CardDef = CardDef {
    name: "Token Faces Test Beast",
    has_cost: false,
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(4),
    toughness: Some(4),
    ..CardDef::BLANK
};
static VILLAGER: CardDef = CardDef {
    name: "Token Faces Test Villager",
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(1),
    toughness: Some(1),
    back: Some(&BEAST),
    ..CardDef::BLANK
};

/// A sorcery front face with a creature back face (Esper Origins' shape).
static RITUAL: CardDef = CardDef {
    name: "Token Faces Test Ritual",
    types: TypeSet::of(&[CardType::Sorcery]),
    back: Some(&BEAST_TWO),
    ..CardDef::BLANK
};
static BEAST_TWO: CardDef = CardDef {
    name: "Token Faces Test Second Beast",
    has_cost: false,
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(5),
    toughness: Some(5),
    ..CardDef::BLANK
};

/// "Enters with a +1/+1 counter on it."
fn with_a_counter(_: &mut dyn cantrip_api::Game, entering: &mut Entering) -> Result<(), Suspend> {
    entering.counters[CounterKind::PlusOne as usize] += 1;
    Ok(())
}

/// A Siege battle front face that enters with a +1/+1 counter, with a creature back face (Belligerent Regisaur's shape,
/// the "enters with" a Braided Net's).
static SIEGE: CardDef = CardDef {
    name: "Token Faces Test Siege",
    types: TypeSet::of(&[CardType::Battle]),
    subtypes: SubtypeSet::of(&[Subtype::Siege]),
    defense: Some(3),
    enters: Some(with_a_counter),
    back: Some(&RAPTOR),
    ..CardDef::BLANK
};
static RAPTOR: CardDef = CardDef {
    name: "Token Faces Test Raptor",
    has_cost: false,
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(3),
    toughness: Some(3),
    ..CardDef::BLANK
};

/// A daybound front face whose back face enters with a +1/+1 counter.
static PUP: CardDef = CardDef {
    name: "Token Faces Test Pup",
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(1),
    toughness: Some(1),
    keywords: Keywords::of(&[Keyword::Daybound]),
    back: Some(&HOUND),
    ..CardDef::BLANK
};
static HOUND: CardDef = CardDef {
    name: "Token Faces Test Hound",
    has_cost: false,
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(4),
    toughness: Some(4),
    keywords: Keywords::of(&[Keyword::Nightbound]),
    enters: Some(with_a_counter),
    ..CardDef::BLANK
};
static BEAR: CardDef = CardDef {
    name: "Token Faces Test Bear",
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(2),
    toughness: Some(2),
    ..CardDef::BLANK
};

/// A double-faced token of its own (an Incubator, CR 111.10i).
static INCUBATOR: CardDef = CardDef {
    name: "Token Faces Test Incubator",
    types: TypeSet::of(&[CardType::Artifact]),
    subtypes: SubtypeSet::of(&[Subtype::Incubator]),
    back: Some(&PHYREXIAN),
    ..CardDef::BLANK
};
static PHYREXIAN: CardDef = CardDef {
    name: "Token Faces Test Phyrexian",
    has_cost: false,
    types: TypeSet::of(&[CardType::Artifact, CardType::Creature]),
    subtypes: SubtypeSet::of(&[Subtype::Phyrexian]),
    power: Some(0),
    toughness: Some(0),
    ..CardDef::BLANK
};

fn game() -> (Game, &'static CardTable) {
    let table: &'static CardTable = Box::leak(Box::new(CardTable::new([&VILLAGER, &RITUAL, &SIEGE, &PUP, &BEAR, &INCUBATOR])));
    let mut game = Game::new(table, &Setup { decks: vec![Vec::new(), Vec::new()], ..Default::default() });
    game.state.update_turn(|t| {
        t.active = YOU;
        t.priority = YOU;
        t.set_step(Step::PrecombatMain);
    });
    (game, table)
}

fn cx(game: &mut Game) -> Cx<'_> {
    Cx { g: game, frame: usize::MAX, cursor: 0, mana_source: None, entering: None, lineage: Vec::new(), damage: None, asked_at: None, doer: None, producing: None }
}

/// The copiable values of the Villager transformed - its back face up, a double-faced permanent's - with "except its name
/// is Copycat": the token is the Villager's card back face up, the Beast under the exception, and it transforms to the
/// Villager, still named Copycat. Its definition is the face up, not the front face (`CardTable::current`).
#[test]
fn a_token_copy_of_a_transformed_double_faced_permanent_has_both_faces_under_its_exceptions() {
    let (mut game, table) = game();
    let values = Copiable::of(table.id_of(&BEAST)).from_faces(Some(table.id_of(&VILLAGER)), true).named("Copycat");
    let mut cx = cx(&mut game);
    let token = ops::create_token_copies(&mut cx, values, YOU, 1).expect("nothing is asked")[0];
    let obj = *cx.s().obj(token);
    assert_eq!(obj.card, table.id_of(&VILLAGER));
    assert!(obj.transformed);
    assert_eq!(cx.cards().current(&obj), table.id_of(&BEAST));
    assert_eq!(cx.chars(token).name, "Copycat");
    assert_eq!(cx.chars(token).power, Some(4));
    assert!(ops::turn_over(&mut cx, token).expect("nothing is asked"));
    let obj = *cx.s().obj(token);
    assert_eq!(cx.cards().current(&obj), table.id_of(&VILLAGER));
    assert_eq!(cx.chars(token).name, "Copycat");
    assert_eq!(cx.chars(token).power, Some(1));
}

/// The copiable values of the Villager's front face alone (a Clone's that copied it): the token has that one face, and
/// does not transform (CR 701.27c).
#[test]
fn a_token_copy_of_one_face_is_single_faced() {
    let (mut game, table) = game();
    let values = Copiable::of(table.id_of(&VILLAGER));
    let mut cx = cx(&mut game);
    let token = ops::create_token_copies(&mut cx, values, YOU, 1).expect("nothing is asked")[0];
    assert!(!ops::turn_over(&mut cx, token).expect("nothing is asked"));
    assert!(!cx.s().obj(token).transformed);
    assert_eq!(cx.chars(token).power, Some(1));
}

/// Made at night, a token copy of a transformed double-faced permanent whose front face has no daybound (Kiki-Jiki on
/// Summon: Esper Maduin), with an exception, enters back face up (CR 707.8a): daybound only ever turns an entry over, never
/// back. Its front face is a sorcery, so face up it could not be created at all (CR 111.5).
#[test]
fn a_token_copy_of_a_back_face_made_at_night_with_an_exception_enters_back_face_up() {
    let (mut game, table) = game();
    game.state.update_turn(|t| t.day_night = DayNight::Night);
    let values = Copiable::of(table.id_of(&BEAST_TWO)).from_faces(Some(table.id_of(&RITUAL)), true).with_keyword(Keyword::Haste);
    let mut cx = cx(&mut game);
    let made = ops::create_token_copies(&mut cx, values, YOU, 1).expect("nothing is asked");
    assert_eq!(made.len(), 1);
    assert!(cx.s().obj(made[0]).transformed);
    assert_eq!(cx.chars(made[0]).power, Some(5));
    assert!(cx.chars(made[0]).has(Keyword::Haste));
}

/// A token copy of a transformed double-faced permanent enters as the face that is up (CR 707.8a): the front face's own
/// "enters with" replacement does not apply to it, and it is no battle, so it has no protector and no defense counters.
#[test]
fn a_token_copy_of_a_back_face_has_none_of_its_front_faces_entering() {
    let (mut game, table) = game();
    let values = Copiable::of(table.id_of(&RAPTOR)).from_faces(Some(table.id_of(&SIEGE)), true).named("Copied Raptor");
    let mut cx = cx(&mut game);
    let token = ops::create_token_copies(&mut cx, values, YOU, 1).expect("nothing is asked")[0];
    assert!(cx.s().obj(token).transformed);
    assert!(!cx.chars(token).is(CardType::Battle));
    assert_eq!(cx.s().obj(token).marks.protector, None);
    assert!(!cx.s().obj(token).has_counters());
}

/// A token whose copy values are a copy effect over both faces of a daybound double-faced card (a copy of a daybound
/// Glasspool-Mimic-like card that copied a Bear): it is a Bear, with no daybound, so at night it enters front face up, and
/// none of its back face's entering replacements apply (CR 702.145b, 707.8a).
#[test]
fn a_token_copy_over_a_daybound_card_enters_as_its_copy_at_night() {
    let (mut game, table) = game();
    game.state.update_turn(|t| t.day_night = DayNight::Night);
    let values = Copiable::of(table.id_of(&BEAR)).from_faces(Some(table.id_of(&PUP)), false);
    let mut cx = cx(&mut game);
    let token = ops::create_token_copies(&mut cx, values, YOU, 1).expect("nothing is asked")[0];
    let obj = *cx.s().obj(token);
    assert_eq!(obj.card, table.id_of(&PUP));
    assert!(!obj.transformed);
    assert!(!obj.has_counters());
    assert_eq!(cx.chars(token).name, BEAR.name);
    // Still double-faced: it transforms, and stays the Bear (CR 707.2).
    assert!(ops::turn_over(&mut cx, token).expect("nothing is asked"));
    assert_eq!(cx.chars(token).name, BEAR.name);
}

/// A token whose copy values are a copy effect over both faces of a card whose front face enters with a counter (a copy
/// of Glasspool Mimic copying a Bear, whose own "may enter as a copy" is no longer its to apply): it enters as the Bear,
/// with none of that face's entering replacements.
#[test]
fn a_token_copy_over_a_double_faced_card_has_none_of_its_cards_entering() {
    let (mut game, table) = game();
    let values = Copiable::of(table.id_of(&BEAR)).from_faces(Some(table.id_of(&SIEGE)), false);
    let mut cx = cx(&mut game);
    let token = ops::create_token_copies(&mut cx, values, YOU, 1).expect("nothing is asked")[0];
    assert_eq!(cx.s().obj(token).card, table.id_of(&SIEGE));
    assert!(!cx.s().obj(token).has_counters());
    assert_eq!(cx.chars(token).name, BEAR.name);
}

/// An Incubator token made by an ability whose text was changed (Sleight of Mind on Sunfall) is still double-faced: it
/// transforms into its Phyrexian back face (CR 111.10i, 701.27a).
#[test]
fn a_double_faced_token_made_with_changed_words_transforms() {
    let (mut game, table) = game();
    let words = Words::NONE.changed(Word::Color(Color::White), Word::Color(Color::Blue));
    let mut cx = cx(&mut game);
    let token = ops::create_tokens_worded(&mut cx, &[(table.id_of(&INCUBATOR), YOU)], false, None, &[], words, None, None).expect("nothing is asked")[0];
    assert!(ops::turn_over(&mut cx, token).expect("nothing is asked"));
    assert!(cx.chars(token).is(CardType::Creature));
}
