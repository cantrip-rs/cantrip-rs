//! Ordered unnamed token names are copiable characteristics, fixed by creating text (CR 111.4, 612.2a, 612.4,
//! 707.2). Subsequent source/text/type changes do not rewrite the name the token was created with.

use crate::{Cx, Game, Setup, ops};
use cantrip_api::*;

const YOU: PlayerId = PlayerId(0);
const THEM: PlayerId = PlayerId(1);

static TOKEN: CardDef = CardDef {
    name: "Forest Tentacle Token", token_name: Some(&[Subtype::Forest, Subtype::Tentacle]),
    has_cost: false, colors: Some(ColorSet::of(Color::Green)),
    types: TypeSet::of(&[CardType::Land, CardType::Creature]),
    subtypes: SubtypeSet::of(&[Subtype::Forest, Subtype::Tentacle]), power: Some(3), toughness: Some(3),
    ..CardDef::BLANK
};
static NAMED: CardDef = CardDef {
    name: "Tentacle", token_name: None,
    ..TOKEN
};
static SOURCE: CardDef = CardDef { name: "Token Name Test Source", types: TypeSet::of(&[CardType::Enchantment]), ..CardDef::BLANK };

fn cx(game: &mut Game) -> Cx<'_> {
    Cx { g: game, frame: usize::MAX, cursor: 0, mana_source: None, entering: None, lineage: Vec::new(), asked_at: None, doer: None }
}

fn game() -> (Game, ObjectId) {
    let table = Box::leak(Box::new(CardTable::new([&TOKEN, &NAMED, &SOURCE])));
    let mut game = Game::new(table, &Setup { decks: vec![Vec::new(), Vec::new()], ..Default::default() });
    let source = game.state.push_obj(Obj::new(table.id_of(&SOURCE), YOU, Zone::BATTLEFIELD));
    game.state.zone_push(Zone::BATTLEFIELD, source);
    (game, source)
}

fn changed() -> Words {
    Words::NONE
        .changed(Word::Subtype(Subtype::Forest), Word::Subtype(Subtype::Island))
        .changed(Word::Subtype(Subtype::Tentacle), Word::Subtype(Subtype::Goblin))
        .changed(Word::Color(Color::Green), Word::Color(Color::Blue))
}

#[test]
fn compound_token_keeps_creating_order_color_and_land_type_with_intrinsic_mana() {
    let (mut game, source) = game();
    let card = game.cards.id_of(&TOKEN);
    let token = ops::create_tokens_worded(&mut cx(&mut game), &[(card, YOU)], false, None, &[], changed(), None).unwrap()[0];
    let c = game.chars(token);
    assert_eq!(c.name, "Island Goblin Token");
    assert!(c.subtypes.has(Subtype::Island) && c.subtypes.has(Subtype::Goblin) && !c.subtypes.has(Subtype::Forest));
    assert_eq!(c.colors, ColorSet::of(Color::Blue));
    assert!(!c.supertypes.basic);
    assert_eq!(c.abilities().count(), 1);
    let AbilityDef::Activated(mana) = game.cards.ability(c.abilities().next().unwrap()) else { panic!() };
    assert_eq!(mana.adds.get(Mana::U), 1);
    // Changing that token's own subtype is not changing the creating text, so it leaves the name alone (CR 111.4).
    cantrip_api::Game::add_effect(&mut cx(&mut game), source, token, EffectKind::ChangeText { from: Word::Subtype(Subtype::Goblin), to: Word::Subtype(Subtype::Elf) }, Duration::Indefinite);
    assert!(game.chars(token).subtypes.has(Subtype::Elf));
    assert_eq!(game.chars(token).name, "Island Goblin Token");
    let values = Query::copiable_values(&game, token);
    let copied = ops::create_token_copies(&mut cx(&mut game), values, THEM, 1).unwrap()[0];
    assert!(game.chars(copied).subtypes.has(Subtype::Goblin) && !game.chars(copied).subtypes.has(Subtype::Elf), "later layer-3 type is not copiable");
    assert_eq!(game.chars(copied).name, "Island Goblin Token");
    cantrip_api::Game::add_effect(&mut cx(&mut game), source, token, EffectKind::SetName("Another literal name"), Duration::Indefinite);
    assert_eq!(game.chars(token).name, "Another literal name");
    let values = Query::copiable_values(&game, token);
    let copied = ops::create_token_copies(&mut cx(&mut game), values, THEM, 1).unwrap()[0];
    assert_eq!(game.chars(copied).name, "Island Goblin Token", "layer-3 SetName is not a copy exception");
}

#[test]
fn copying_changed_token_freezes_name_before_copy_exceptions_or_later_changes() {
    let (mut game, source) = game();
    let card = game.cards.id_of(&TOKEN);
    let token = ops::create_tokens_worded(&mut cx(&mut game), &[(card, YOU)], false, None, &[], changed(), None).unwrap()[0];
    let values = Query::copiable_values(&game, token);
    let copied = ops::create_token_copies(&mut cx(&mut game), values, THEM, 1).unwrap()[0];
    assert_eq!(game.chars(copied).name, game.chars(token).name);
    assert_eq!(game.state.obj(copied).owner, THEM);
    let other_type = ops::create_token_copies(&mut cx(&mut game), values.typed(TypeSet::of(&[CardType::Creature]), SubtypeSet::of(&[Subtype::Frog])), THEM, 1).unwrap()[0];
    assert_eq!(game.chars(other_type).name, "Island Goblin Token");
    assert!(game.chars(other_type).subtypes.has(Subtype::Frog));
    let renamed = ops::create_token_copies(&mut cx(&mut game), values.named("A literal Tentacle"), THEM, 1).unwrap()[0];
    assert_eq!(game.chars(renamed).name, "A literal Tentacle");
    cantrip_api::Game::gain_control(&mut cx(&mut game), source, copied, YOU, Duration::Indefinite);
    assert_eq!(game.chars(copied).name, "Island Goblin Token");
    let gone = ops::return_to_hand(&mut cx(&mut game), &[copied]);
    assert!(!gone.is_empty());
    crate::settle::state_based_actions_alone(&mut cx(&mut game)).unwrap();
    assert!(game.state.zone(Zone::hand(THEM)).is_empty(), "returned token should cease, not become card");
}

#[test]
fn explicitly_named_subtype_word_is_not_rewritten_but_its_assigned_types_are() {
    let (mut game, _) = game();
    let card = game.cards.id_of(&NAMED);
    let token = ops::create_tokens_worded(&mut cx(&mut game), &[(card, YOU)], false, None, &[], changed(), None).unwrap()[0];
    assert_eq!(game.chars(token).name, "Tentacle");
    assert!(game.chars(token).subtypes.has(Subtype::Goblin));
}

#[test]
fn unchanged_ordered_name_stays_fixed_when_later_types_change() {
    let (mut game, source) = game();
    let card = game.cards.id_of(&TOKEN);
    let token = ops::create_tokens(&mut cx(&mut game), card, YOU, 1, false).unwrap()[0];
    assert_eq!(game.chars(token).name, "Forest Tentacle Token");
    assert!(!game.chars(token).supertypes.basic);
    let values = Query::copiable_values(&game, token);
    let copied = ops::create_token_copies(&mut cx(&mut game), values.typed(TypeSet::of(&[CardType::Creature]), SubtypeSet::of(&[Subtype::Frog])), THEM, 1).unwrap()[0];
    assert_eq!(game.chars(copied).name, "Forest Tentacle Token");
    cantrip_api::Game::add_effect(&mut cx(&mut game), source, token, EffectKind::ChangeText { from: Word::Subtype(Subtype::Tentacle), to: Word::Subtype(Subtype::Goblin) }, Duration::Indefinite);
    assert_eq!(game.chars(token).name, "Forest Tentacle Token");
    assert!(game.chars(token).subtypes.has(Subtype::Goblin));
}
