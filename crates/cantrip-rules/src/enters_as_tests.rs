//! An effect that puts a card onto the battlefield as something it says (`Game::put_onto_battlefield_as`, `EntersAs`): the
//! card enters as that (CR 614.12) - an Aura enchantment with the enchant ability the effect gives it, never a creature - and
//! as an Aura chooses what it enchants as it enters, without targeting (CR 303.4f), stays where it is with nothing to enchant
//! (CR 303.4g), and is put into its owner's graveyard once what it enchants is no longer legal (CR 303.4c, 704.5m). Over test
//! cards shaped like Old-Growth Troll's "return it to the battlefield. It's an Aura enchantment with enchant Forest you
//! control and '...'".

use crate::{Cx, Game, Setup};
use cantrip_api::*;
use cantrip_state::{Frame, FrameKind};

const YOU: PlayerId = PlayerId(0);
const THEM: PlayerId = PlayerId(1);

static FILLER: CardDef = CardDef { name: "Enters As Filler", has_cost: false, types: TypeSet::of(&[CardType::Land]), ..CardDef::BLANK };

static FOREST: CardDef = CardDef {
    name: "Enters As Forest",
    has_cost: false,
    types: TypeSet::of(&[CardType::Land]),
    subtypes: SubtypeSet::of(&[Subtype::Forest]),
    ..CardDef::BLANK
};

/// "Enchant Forest you control" (CR 702.5a): judged for the Aura's controller.
fn forest_you_control(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    matches!(t, Target::Object(id) if q.is_on_battlefield(id) && q.chars(id).has_subtype(Subtype::Forest) && q.controller(id) == asker.controller)
}

fn nothing(_: &mut dyn cantrip_api::Game, _: &StackItem) -> Result<(), Suspend> {
    Ok(())
}

fn never(_: &dyn Query, _: ObjectId, _: &Event) -> Option<Fired> {
    None
}

/// A 4/4 Troll Warrior with trample whose effect returns it as "an Aura enchantment with enchant Forest you control and
/// '{T}: Add {G}{G}.'"
static TROLL: CardDef = CardDef {
    name: "Enters As Troll",
    types: TypeSet::of(&[CardType::Creature]),
    subtypes: SubtypeSet::of(&[Subtype::Troll, Subtype::Warrior]),
    power: Some(4),
    toughness: Some(4),
    keywords: Keywords::of(&[Keyword::Trample]),
    abilities: &[triggered(cantrip_api::when(EventMask::ZONE_CHANGE, never), nothing, "When this creature dies, return it.")],
    becomes_aura_enchant: Some(target(forest_you_control)),
    delayed: &[mana_ability(TAP, add(Mana::G), "{T}: Add {G}{G}.")],
    ..CardDef::BLANK
};

/// An Enchantment Creature - Saga Troll that returns as the Troll does.
static SAGA_TROLL: CardDef = CardDef {
    name: "Enters As Saga Troll",
    types: TypeSet::of(&[CardType::Enchantment, CardType::Creature]),
    subtypes: SubtypeSet::of(&[Subtype::Saga, Subtype::Troll]),
    ..TROLL
};

/// A Sorcery - Lesson.
static LESSON: CardDef = CardDef {
    name: "Enters As Lesson",
    types: TypeSet::of(&[CardType::Sorcery]),
    subtypes: SubtypeSet::of(&[Subtype::Lesson]),
    ..CardDef::BLANK
};

/// A plain creature card: what an effect puts onto the battlefield as an Aura of another card's words.
static BEAR: CardDef = CardDef {
    name: "Enters As Bear",
    types: TypeSet::of(&[CardType::Creature]),
    subtypes: SubtypeSet::of(&[Subtype::Bear]),
    power: Some(2),
    toughness: Some(2),
    ..CardDef::BLANK
};

fn creature_on_the_battlefield(q: &dyn Query, _: &Asker, t: Target) -> bool {
    matches!(t, Target::Object(id) if q.is_on_battlefield(id) && q.chars(id).is(CardType::Creature))
}

/// An Aura that prints "enchant creature" and whose effect makes another card an Aura with "enchant Forest you control".
static TWO_ENCHANTS: CardDef = CardDef {
    name: "Enters As Two Enchants",
    types: TypeSet::of(&[CardType::Enchantment]),
    subtypes: SubtypeSet::of(&[Subtype::Aura]),
    enchant: Some(target(creature_on_the_battlefield)),
    becomes_aura_enchant: Some(target(forest_you_control)),
    abilities: &[triggered(cantrip_api::when(EventMask::ZONE_CHANGE, never), nothing, "When this dies, return a creature card as an Aura.")],
    ..CardDef::BLANK
};

/// "Enchant opponent" (CR 303.4, 702.5d): a player, one its controller's opponent.
fn opponent(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    matches!(t, Target::Player(p) if p != asker.controller && !q.state().player(p).lost)
}

/// An Aura Curse with "enchant opponent".
static CURSE: CardDef = CardDef {
    name: "Enters As Curse",
    types: TypeSet::of(&[CardType::Enchantment]),
    subtypes: SubtypeSet::of(&[Subtype::Aura, Subtype::Curse]),
    enchant: Some(target(opponent)),
    ..CardDef::BLANK
};

/// "It's an Aura enchantment" with the enchant ability of the effect's card and no other ability.
static AS_BARE_AURA: EntersAs = EntersAs { aura: true, ..EntersAs::NONE };

/// An Aura enchantment that names an ability its card does not list.
static AS_AURA_NAMING_TOO_MUCH: EntersAs = EntersAs { aura: true, abilities: &[3], ..EntersAs::NONE };

static AS_ENCHANTMENT: EntersAs = EntersAs { types: Some(TypeSet::of(&[CardType::Enchantment])), ..EntersAs::NONE };

static AS_AURA: EntersAs = EntersAs { aura: true, abilities: &[0], ..EntersAs::NONE };

/// "Whenever a creature enters, ...": what would see the returning card if it entered as a creature.
fn a_creature_enters(q: &dyn Query, _: ObjectId, event: &Event) -> Option<Fired> {
    match event {
        Event::ZoneChange { new, to, .. } if to.is(ZoneKind::Battlefield) && q.chars(*new).is(CardType::Creature) => Fired::PLAIN,
        _ => None,
    }
}

static WATCHER: CardDef = CardDef {
    name: "Enters As Watcher",
    types: TypeSet::of(&[CardType::Enchantment]),
    abilities: &[triggered(cantrip_api::when(EventMask::ZONE_CHANGE, a_creature_enters), nothing, "Whenever a creature enters, nothing.")],
    ..CardDef::BLANK
};

fn game() -> Game {
    let table: &'static CardTable = Box::leak(Box::new(CardTable::new([&FILLER, &FOREST, &TROLL, &WATCHER, &SAGA_TROLL, &LESSON, &BEAR, &TWO_ENCHANTS, &CURSE, &EQUIPPED_BEAR])));
    let deck = || vec![table.id_of(&FILLER); 20];
    let mut g = Game::new(table, &Setup { decks: vec![deck(), deck()], ..Default::default() });
    while !g.state.agenda.is_empty() {
        g.state.frame_remove(0);
    }
    g.state.update_turn(|t| {
        t.active = YOU;
        t.priority = YOU;
        t.set_step(Step::PrecombatMain);
    });
    g.state.frame_push(Frame::new(FrameKind::Priority));
    g.state.update_player(YOU, |p| p.turn_began = 1);
    g
}

fn put(g: &mut Game, def: &'static CardDef, owner: PlayerId, zone: Zone) -> ObjectId {
    let id = g.state.push_obj(Obj::new(g.cards.id_of(def), owner, zone));
    let timestamp = g.state.timestamp();
    g.state.update_obj(id, |o| o.timestamp = timestamp);
    g.state.zone_push(zone, id);
    g.state.see_statics();
    id
}

fn cx(game: &mut Game) -> Cx<'_> {
    Cx { g: game, frame: usize::MAX, cursor: 0, mana_source: None, entering: None, lineage: Vec::new(), asked_at: None, doer: None, producing: None }
}

/// The Troll's ability returning the Troll card `card` from YOU's graveyard as the Aura.
fn return_as_aura(g: &mut Game, card: ObjectId) -> Option<ObjectId> {
    let ability = AbilityRef { card: g.cards.id_of(&TROLL), index: 0 };
    let item = crate::actions::stack_item(StackKind::Triggered, card, ability, YOU, &[]);
    cantrip_api::Game::put_onto_battlefield_as(&mut cx(g), card, YOU, &item, &AS_AURA).expect("one host: nothing to ask")
}

/// It enters attached to the one Forest its controller controls, as an Aura enchantment that is no creature - nothing that
/// watches creatures enter sees it - with its own trample and the ability it is given.
#[test]
fn it_enters_attached_to_a_legal_host_as_a_non_creature_aura() {
    let mut g = game();
    put(&mut g, &WATCHER, YOU, Zone::BATTLEFIELD);
    let forest = put(&mut g, &FOREST, YOU, Zone::BATTLEFIELD);
    put(&mut g, &FOREST, THEM, Zone::BATTLEFIELD);
    let card = put(&mut g, &TROLL, YOU, Zone::graveyard(YOU));
    let aura = return_as_aura(&mut g, card).expect("it entered");
    let o = *g.state.obj(aura);
    assert!(o.zone.is(ZoneKind::Battlefield));
    assert_eq!(o.attached_to, Some(forest));
    let q: &dyn Query = &cx(&mut g);
    let chars = q.chars(aura);
    assert!(chars.is(CardType::Enchantment) && !chars.is(CardType::Creature));
    assert!(chars.has_subtype(Subtype::Aura) && !chars.has_subtype(Subtype::Troll) && !chars.has_subtype(Subtype::Warrior));
    assert!(chars.has(Keyword::Trample));
    assert!(chars.abilities.contains(&Some(AbilityRef::delayed(g.cards.id_of(&TROLL), 0))));
    assert!(g.state.triggers.is_empty(), "it never entered as a creature");
    assert!(g.log.iter().any(|r| matches!(r.event, Event::Attached { attachment, to: Target::Object(host) } if attachment == aura && host == forest)));

    // The watcher does see a creature enter: the check above is a real one.
    let other = put(&mut g, &TROLL, YOU, Zone::graveyard(YOU));
    cantrip_api::Game::put_onto_battlefield(&mut cx(&mut g), other, YOU, false).expect("nothing to ask");
    assert!(!g.state.triggers.is_empty());
}

/// With no Forest its controller controls, it stays in the graveyard (CR 303.4g): an opponent's Forest is no legal host.
#[test]
fn with_no_legal_host_it_stays_in_the_graveyard() {
    let mut g = game();
    put(&mut g, &FOREST, THEM, Zone::BATTLEFIELD);
    let card = put(&mut g, &TROLL, YOU, Zone::graveyard(YOU));
    assert_eq!(return_as_aura(&mut g, card), None);
    assert!(g.state.obj(card).zone.is(ZoneKind::Graveyard));
    assert!(g.state.obj(card).became.is_none(), "the card did not move");
    assert!(g.state.effects.is_empty(), "nothing was given the card");
}

/// Once the Forest it enchants is no longer one its controller controls, the state-based actions put it into its owner's
/// graveyard (CR 303.4c, 704.5m): the enchant ability it was given goes on judging its host.
#[test]
fn it_falls_off_when_its_host_is_no_longer_legal() {
    let mut g = game();
    let forest = put(&mut g, &FOREST, YOU, Zone::BATTLEFIELD);
    let card = put(&mut g, &TROLL, YOU, Zone::graveyard(YOU));
    let aura = return_as_aura(&mut g, card).expect("it entered");
    assert!(!crate::settle::state_based_actions_alone(&mut cx(&mut g)).unwrap_or(true), "attached legally, it stays");
    assert!(g.state.battlefield().contains(&aura));

    cantrip_api::Game::add_effect(&mut cx(&mut g), forest, forest, EffectKind::Control(THEM), Duration::Indefinite);
    assert!(crate::settle::state_based_actions_alone(&mut cx(&mut g)).unwrap_or(false));
    assert!(!g.state.battlefield().contains(&aura));
    let now = g.state.obj(aura).became.expect("it moved");
    assert_eq!(g.state.obj(now).zone, Zone::graveyard(YOU));
}

/// Its enchant ability is one of its abilities (CR 702.5a, 613.1f): once a later effect takes all its abilities, nothing
/// restricts what it enchants, and a Forest no longer its controller's is not illegal on that ground (CR 303.4c, 704.5m).
#[test]
fn an_aura_that_lost_its_enchant_stays_when_its_host_stops_matching() {
    let mut g = game();
    let forest = put(&mut g, &FOREST, YOU, Zone::BATTLEFIELD);
    let card = put(&mut g, &TROLL, YOU, Zone::graveyard(YOU));
    let aura = return_as_aura(&mut g, card).expect("it entered");
    cantrip_api::Game::add_effect(&mut cx(&mut g), aura, aura, EffectKind::LoseAllAbilities, Duration::Indefinite);
    assert!(!cx(&mut g).chars(aura).abilities().any(|a| a.is_enchant()), "the enchant ability is lost with the rest");
    cantrip_api::Game::add_effect(&mut cx(&mut g), forest, forest, EffectKind::Control(THEM), Duration::Indefinite);
    assert!(!crate::settle::state_based_actions_alone(&mut cx(&mut g)).unwrap_or(true));
    assert!(g.state.battlefield().contains(&aura));
    assert_eq!(g.state.obj(aura).attached_to, Some(forest));
}

/// An Aura that is also a creature can enchant nothing (CR 303.4d): made a creature, it is put into its owner's graveyard.
#[test]
fn an_aura_that_becomes_a_creature_falls_off() {
    let mut g = game();
    put(&mut g, &FOREST, YOU, Zone::BATTLEFIELD);
    let card = put(&mut g, &TROLL, YOU, Zone::graveyard(YOU));
    let aura = return_as_aura(&mut g, card).expect("it entered");
    cantrip_api::Game::add_effect(&mut cx(&mut g), aura, aura, EffectKind::AddTypes(TypeSet::of(&[CardType::Creature])), Duration::Indefinite);
    assert!(crate::settle::state_based_actions_alone(&mut cx(&mut g)).unwrap_or(false));
    assert!(!g.state.battlefield().contains(&aura));
}

/// "It's an enchantment" keeps the subtypes of the card type it still has (CR 205.1a): an Enchantment Creature - Saga Troll
/// returns an Enchantment - Saga, its creature type gone with the creature type.
#[test]
fn set_types_keep_the_subtypes_of_a_type_it_still_has() {
    let mut g = game();
    let card = put(&mut g, &SAGA_TROLL, YOU, Zone::graveyard(YOU));
    let ability = AbilityRef { card: g.cards.id_of(&SAGA_TROLL), index: 0 };
    let item = crate::actions::stack_item(StackKind::Triggered, card, ability, YOU, &[]);
    let back = cantrip_api::Game::put_onto_battlefield_as(&mut cx(&mut g), card, YOU, &item, &AS_ENCHANTMENT).expect("nothing asked");
    let chars = cx(&mut g).chars(back.expect("it entered"));
    assert_eq!(chars.types, TypeSet::of(&[CardType::Enchantment]));
    assert_eq!(chars.subtypes, SubtypeSet::of(&[Subtype::Saga]));
}

/// "It's an Aura enchantment" sets its enchantment type too (CR 205.1a): the Saga Troll returns an Enchantment - Aura alone.
#[test]
fn an_aura_enchantment_has_no_other_enchantment_type() {
    let mut g = game();
    put(&mut g, &FOREST, YOU, Zone::BATTLEFIELD);
    let card = put(&mut g, &SAGA_TROLL, YOU, Zone::graveyard(YOU));
    let ability = AbilityRef { card: g.cards.id_of(&SAGA_TROLL), index: 0 };
    let item = crate::actions::stack_item(StackKind::Triggered, card, ability, YOU, &[]);
    let back = cantrip_api::Game::put_onto_battlefield_as(&mut cx(&mut g), card, YOU, &item, &AS_AURA).expect("one host");
    let chars = cx(&mut g).chars(back.expect("it entered"));
    assert_eq!(chars.types, TypeSet::of(&[CardType::Enchantment]));
    assert_eq!(chars.subtypes, SubtypeSet::of(&[Subtype::Aura]));
}

/// An instant or sorcery whose card types are set keeps that type and its spell types (CR 205.1a).
#[test]
fn set_types_leave_a_sorcery_a_sorcery() {
    let mut g = game();
    let lesson = put(&mut g, &LESSON, YOU, Zone::BATTLEFIELD);
    cantrip_api::Game::add_effect(&mut cx(&mut g), lesson, lesson, EffectKind::SetTypes(TypeSet::of(&[CardType::Artifact])), Duration::Indefinite);
    let chars = cx(&mut g).chars(lesson);
    assert_eq!(chars.types, TypeSet::of(&[CardType::Artifact, CardType::Sorcery]));
    assert_eq!(chars.subtypes, SubtypeSet::of(&[Subtype::Lesson]));
}

/// A card that prints one enchant ability and gives another keeps them apart (CR 702.5a): the Bear its effect returns as an
/// Aura has the enchant ability the effect gives, "enchant Forest you control", not the card's own "enchant creature".
#[test]
fn a_given_enchant_is_never_the_cards_printed_one() {
    let mut g = game();
    let forest = put(&mut g, &FOREST, YOU, Zone::BATTLEFIELD);
    put(&mut g, &BEAR, YOU, Zone::BATTLEFIELD);
    let source = put(&mut g, &TWO_ENCHANTS, YOU, Zone::graveyard(YOU));
    let card = put(&mut g, &BEAR, YOU, Zone::graveyard(YOU));
    let ability = AbilityRef { card: g.cards.id_of(&TWO_ENCHANTS), index: 0 };
    let item = crate::actions::stack_item(StackKind::Triggered, source, ability, YOU, &[]);
    let aura = cantrip_api::Game::put_onto_battlefield_as(&mut cx(&mut g), card, YOU, &item, &AS_BARE_AURA).expect("one host").expect("it entered");
    assert_eq!(g.state.obj(aura).attached_to, Some(forest));
    let given = AbilityRef::enchant_given(g.cards.id_of(&TWO_ENCHANTS));
    let abilities: Vec<AbilityRef> = cx(&mut g).chars(aura).abilities().filter(|a| a.is_enchant()).collect();
    assert_eq!(abilities, vec![given]);
}

/// "It loses '[its printed enchant]' and gains '[another]'" (CR 113.10, 613.1f; Animate Dead): the printed enchant ability is
/// gone, every other ability stays in its order, and the gained one alone judges the host (CR 303.4c, 704.5m). A creature it
/// enchants is no longer legal (it was only by the lost ability), so it is put into its owner's graveyard.
#[test]
fn an_aura_that_loses_its_printed_enchant_is_judged_by_the_one_it_gains() {
    let mut g = game();
    let bear = put(&mut g, &BEAR, YOU, Zone::BATTLEFIELD);
    put(&mut g, &FOREST, YOU, Zone::BATTLEFIELD);
    let aura = put(&mut g, &TWO_ENCHANTS, YOU, Zone::BATTLEFIELD);
    g.state.update_obj(aura, |o| o.attached_to = Some(bear));
    assert!(!crate::settle::state_based_actions_alone(&mut cx(&mut g)).unwrap_or(true), "enchant creature: it stays");

    let card = g.cards.id_of(&TWO_ENCHANTS);
    let mut c = cx(&mut g);
    cantrip_api::Game::add_effect(&mut c, aura, aura, EffectKind::LoseEnchant(card), Duration::Indefinite);
    cantrip_api::Game::add_effect(&mut c, aura, aura, EffectKind::Enchant(card), Duration::Indefinite);
    let abilities: Vec<AbilityRef> = c.chars(aura).abilities().collect();
    assert_eq!(abilities, vec![AbilityRef { card, index: 0 }, AbilityRef::enchant_given(card)]);
    assert!(crate::settle::state_based_actions_alone(&mut cx(&mut g)).unwrap_or(false));
    assert!(!g.state.battlefield().contains(&aura));
}

/// Only the enchant ability it names is lost (CR 113.10b), and at its timestamp (CR 613.7b): losing another card's changes
/// nothing, and an enchant ability given after the loss is kept. With its printed one lost and nothing given, nothing
/// restricts what it enchants (CR 702.5a), so it stays.
#[test]
fn only_the_named_enchant_is_lost_and_one_given_later_is_kept() {
    let mut g = game();
    let bear = put(&mut g, &BEAR, YOU, Zone::BATTLEFIELD);
    let aura = put(&mut g, &TWO_ENCHANTS, YOU, Zone::BATTLEFIELD);
    g.state.update_obj(aura, |o| o.attached_to = Some(bear));
    let (card, curse) = (g.cards.id_of(&TWO_ENCHANTS), g.cards.id_of(&CURSE));
    let mut c = cx(&mut g);
    cantrip_api::Game::add_effect(&mut c, aura, aura, EffectKind::LoseEnchant(curse), Duration::Indefinite);
    assert!(c.chars(aura).abilities().any(|a| a == AbilityRef::enchant(card)), "another card's enchant: nothing to lose");
    cantrip_api::Game::add_effect(&mut c, aura, aura, EffectKind::LoseEnchant(card), Duration::Indefinite);
    assert!(!c.chars(aura).abilities().any(|a| a.is_enchant()), "its printed enchant is lost");
    assert!(!crate::settle::state_based_actions_alone(&mut cx(&mut g)).unwrap_or(true), "restricted by nothing, it stays");

    let mut c = cx(&mut g);
    cantrip_api::Game::add_effect(&mut c, aura, aura, EffectKind::Enchant(card), Duration::Indefinite);
    let enchants: Vec<AbilityRef> = c.chars(aura).abilities().filter(|a| a.is_enchant()).collect();
    assert_eq!(enchants, vec![AbilityRef::enchant_given(card)], "given after the loss, it is kept");
    assert!(crate::settle::state_based_actions_alone(&mut cx(&mut g)).unwrap_or(false), "a Bear is no Forest");
    assert!(!g.state.battlefield().contains(&aura));
}

/// An Aura with "enchant player" put onto the battlefield without being cast is attached to a player its enchant ability
/// allows (CR 303.4f: "a legal object or player"): the Curse enchants the opponent.
#[test]
fn an_aura_put_onto_the_battlefield_can_enchant_a_player() {
    let mut g = game();
    put(&mut g, &FOREST, YOU, Zone::BATTLEFIELD);
    let card = put(&mut g, &CURSE, YOU, Zone::graveyard(YOU));
    let curse = cantrip_api::Game::put_onto_battlefield(&mut cx(&mut g), card, YOU, false).expect("one host");
    let o = *g.state.obj(curse);
    assert!(g.state.battlefield().contains(&curse));
    assert_eq!((o.attached_to, o.attached_player), (None, Some(THEM)));
    assert!(!crate::settle::state_based_actions_alone(&mut cx(&mut g)).unwrap_or(true), "attached legally, it stays");
}

/// An Aura with no enchant ability as it enters - one an effect makes an Aura with none to give - is restricted by none
/// (CR 702.5a): every object and every player is a legal choice for it, but itself (CR 303.4d) and what enters with it.
#[test]
fn an_aura_with_no_enchant_ability_may_enchant_anything() {
    let mut g = game();
    let forest = put(&mut g, &FOREST, YOU, Zone::BATTLEFIELD);
    let theirs = put(&mut g, &FOREST, THEM, Zone::BATTLEFIELD);
    let card = put(&mut g, &BEAR, YOU, Zone::graveyard(YOU));
    let entering = Entering {
        object: card,
        controller: YOU,
        tapped: false,
        counters: [0; CounterKind::COUNT],
        copy: None,
        prepared: false,
        chosen_color: None,
        chosen_subtype: None,
        chosen_player: None,
        chosen_card: None,
        chosen_option: None,
        chosen_object: None,
        attach: None,
        attach_player: None,
        x: 0,
        transformed: false,
        protector: None,
        instead: None,
        becomes: Some(Becomes { what: &AS_BARE_AURA, card: g.cards.id_of(&BEAR), words: Words::NONE }),
        own: None,
    };
    let mut c = cx(&mut g);
    let chars = crate::ops::as_it_would_exist(&mut c, &entering).chars;
    assert!(chars.has_subtype(Subtype::Aura) && !chars.is(CardType::Creature));
    assert!(!chars.abilities().any(|a| a.is_enchant()), "Bear gives no enchant ability");
    let hosts = crate::attach::aura_hosts(&c, card, &chars, YOU, &[card]);
    assert_eq!(hosts, vec![Target::Object(forest), Target::Object(theirs), Target::Player(YOU), Target::Player(THEM)]);
}

/// An `EntersAs` naming a `delayed` ability its card does not list gives nothing for it, and the card still enters as the
/// rest says: no effect is made for an ability that does not exist.
#[test]
fn an_ability_its_card_does_not_list_is_not_given() {
    let mut g = game();
    let forest = put(&mut g, &FOREST, YOU, Zone::BATTLEFIELD);
    let source = put(&mut g, &TWO_ENCHANTS, YOU, Zone::graveyard(YOU));
    let card = put(&mut g, &BEAR, YOU, Zone::graveyard(YOU));
    let ability = AbilityRef { card: g.cards.id_of(&TWO_ENCHANTS), index: 0 };
    let item = crate::actions::stack_item(StackKind::Triggered, source, ability, YOU, &[]);
    let aura =
        cantrip_api::Game::put_onto_battlefield_as(&mut cx(&mut g), card, YOU, &item, &AS_AURA_NAMING_TOO_MUCH).expect("one host").expect("it entered");
    assert_eq!(g.state.obj(aura).attached_to, Some(forest));
    assert!(!g.state.effects.iter().any(|e| matches!(e.kind, EffectKind::AddAbility(_))));
    let chars = cx(&mut g).chars(aura);
    assert!(chars.has_subtype(Subtype::Aura));
}

/// An Equipment artifact creature: its artifact subtype goes too when it enters as a Food.
static EQUIPPED_BEAR: CardDef = CardDef {
    name: "Enters As Equipped Bear",
    types: TypeSet::of(&[CardType::Artifact, CardType::Creature]),
    subtypes: SubtypeSet::of(&[Subtype::Equipment, Subtype::Bear]),
    power: Some(2),
    toughness: Some(2),
    ..CardDef::BLANK
};

/// "They are Food artifacts. (They lose all other types and subtypes.)": several cards enter together under the player given as
/// artifacts with the one subtype Food, never as creatures - a creature watcher sees nothing - untapped.
#[test]
fn several_cards_enter_together_as_food_artifacts() {
    static AS_FOOD: EntersAs = EntersAs {
        types: Some(TypeSet::of(&[CardType::Artifact])),
        loses_subtypes_of: &[CardType::Artifact],
        subtypes: &[Subtype::Food],
        ..EntersAs::NONE
    };
    let mut g = game();
    put(&mut g, &WATCHER, YOU, Zone::BATTLEFIELD);
    let source = put(&mut g, &TWO_ENCHANTS, YOU, Zone::graveyard(YOU));
    let bear = put(&mut g, &BEAR, THEM, Zone::graveyard(THEM));
    let equipped = put(&mut g, &EQUIPPED_BEAR, THEM, Zone::graveyard(THEM));
    let ability = AbilityRef { card: g.cards.id_of(&TWO_ENCHANTS), index: 0 };
    let item = crate::actions::stack_item(StackKind::Spell, source, ability, YOU, &[]);
    let entered = cantrip_api::Game::put_all_onto_battlefield_becoming(&mut cx(&mut g), &[(bear, YOU), (equipped, YOU)], &item, &AS_FOOD, &[])
        .expect("nothing to ask");
    assert_eq!(entered.len(), 2);
    for &p in &entered {
        let chars = cx(&mut g).chars(p);
        assert!(chars.is(CardType::Artifact) && !chars.is(CardType::Creature));
        assert!(chars.has_subtype(Subtype::Food) && !chars.has_subtype(Subtype::Bear) && !chars.has_subtype(Subtype::Equipment));
        assert_eq!(g.state.obj(p).controller, YOU);
        assert!(!g.state.obj(p).tapped);
    }
    assert!(g.state.triggers.is_empty(), "none of them entered as a creature");
}

/// "It's a 3/3 Robot artifact creature with flying": a card enters as that, its power and toughness and keywords from the moment
/// it enters (CR 614.12, 613.1f, 613.4b), a creature as it enters, so what watches creatures enter sees it.
#[test]
fn a_card_enters_with_the_power_toughness_and_keywords_it_is_given() {
    static AS_ROBOT: EntersAs = EntersAs {
        types: Some(TypeSet::of(&[CardType::Artifact, CardType::Creature])),
        subtypes: &[Subtype::Robot],
        power_toughness: Some((3, 3)),
        keywords: Keywords::of(&[Keyword::Flying]),
        ..EntersAs::NONE
    };
    let mut g = game();
    let source = put(&mut g, &TWO_ENCHANTS, YOU, Zone::graveyard(YOU));
    let card = put(&mut g, &EQUIPPED_BEAR, YOU, Zone::graveyard(YOU));
    let ability = AbilityRef { card: g.cards.id_of(&TWO_ENCHANTS), index: 0 };
    let item = crate::actions::stack_item(StackKind::Spell, source, ability, YOU, &[]);
    let entered = cantrip_api::Game::put_onto_battlefield_as(&mut cx(&mut g), card, YOU, &item, &AS_ROBOT).expect("nothing to ask").expect("it enters");
    let chars = cx(&mut g).chars(entered);
    assert!(chars.is(CardType::Artifact) && chars.is(CardType::Creature) && chars.has_subtype(Subtype::Robot));
    assert_eq!((chars.power, chars.toughness), (Some(3), Some(3)));
    assert!(chars.keywords.has(Keyword::Flying));
}
