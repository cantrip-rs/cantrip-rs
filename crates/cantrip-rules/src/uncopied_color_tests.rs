//! "Except it doesn't copy that creature's color" (CR 707.9c, 613.2a, `Copiable::uncopied_color`): the copy's colors are
//! those of the values beneath it, as they are now. Vesuvan Doppelganger is the printed card; these are test fixtures.
use crate::{Cx, Game, Setup};
use cantrip_api::*;

const YOU: PlayerId = PlayerId(0);

static SHIFTER: CardDef = CardDef { name: "Blue Shifter", colors: Some(ColorSet::of(Color::Blue)), types: TypeSet::of(&[CardType::Creature]), power: Some(0), toughness: Some(1), ..CardDef::BLANK };
static BEAR: CardDef = CardDef { name: "Green Bear", colors: Some(ColorSet::of(Color::Green)), types: TypeSet::of(&[CardType::Creature]), power: Some(2), toughness: Some(2), ..CardDef::BLANK };
static LEFT_DOOR: CardDef = CardDef { name: "Red Door", cost: mana("{R}"), types: TypeSet::of(&[CardType::Enchantment]), subtypes: SubtypeSet::of(&[Subtype::Room]), ..CardDef::BLANK };
static RIGHT_DOOR: CardDef = CardDef { name: "Green Door", cost: mana("{G}"), ..LEFT_DOOR };
static ROOM: CardDef = CardDef { name: "Red Door // Green Door", cost: mana("{R}{G}"), split: Some((&LEFT_DOOR, &RIGHT_DOOR)), ..LEFT_DOOR };
static KOBOLD: CardDef = CardDef { name: "Red Kobold", colors: Some(ColorSet::of(Color::Red)), types: TypeSet::of(&[CardType::Creature]), power: Some(0), toughness: Some(1), ..CardDef::BLANK };

fn cx(g: &mut Game) -> Cx<'_> {
    Cx { g, frame: usize::MAX, cursor: 0, mana_source: None, entering: None, lineage: vec![], damage: None, asked_at: None, doer: None, producing: None }
}

fn game() -> Game {
    let cards = Box::leak(Box::new(CardTable::new([&SHIFTER, &BEAR, &KOBOLD, &LEFT_DOOR, &RIGHT_DOOR, &ROOM])));
    let mut g = Game::new(cards, &Setup { decks: vec![vec![], vec![]], ..Default::default() });
    while !g.state.agenda.is_empty() {
        g.state.frame_remove(0);
    }
    g.state.update_turn(|t| {
        t.number = 3;
        t.active = YOU;
        t.priority = YOU;
        t.set_step(Step::PrecombatMain);
    });
    g
}

fn put(g: &mut Game, def: &'static CardDef) -> ObjectId {
    let id = g.state.push_obj(Obj::new(g.cards.id_of(def), YOU, Zone::BATTLEFIELD));
    let stamp = g.state.timestamp();
    g.state.update_obj(id, |o| o.timestamp = stamp);
    g.state.zone_push(Zone::BATTLEFIELD, id);
    id
}

fn colors(g: &Game, id: ObjectId) -> ColorSet {
    crate::layers::chars(g.cards, &g.state, &g.log, id).colors
}

/// The shifter, a copy of the kobold until end of turn (Mirrorweave), then a copy of the bear that doesn't copy color
/// (the Doppelganger's upkeep trigger): returns the shifter and the bear.
fn copied_twice(g: &mut Game) -> (ObjectId, ObjectId) {
    let shifter = put(g, &SHIFTER);
    let bear = put(g, &BEAR);
    let kobold = put(g, &KOBOLD);
    let kobold_values = cx(g).copiable_values(kobold);
    cantrip_api::Game::become_copy(&mut cx(g), kobold, shifter, kobold_values, Duration::EndOfTurn);
    let values = cx(g).copiable_values(bear).uncopied_color();
    cantrip_api::Game::become_copy(&mut cx(g), shifter, shifter, values, Duration::Indefinite);
    (shifter, bear)
}

/// Color left uncopied is the color the earlier copy effect gives, and the object's own again once that effect ends (CR
/// 613.2a, 613.7): not the color it had as the copy began. Taken back through the journal, it is the earlier one's again.
#[test]
fn an_uncopied_color_follows_the_copy_effect_beneath_as_it_ends() {
    let mut g = game();
    let (shifter, bear) = copied_twice(&mut g);
    assert_eq!(crate::layers::chars(g.cards, &g.state, &g.log, shifter).card, g.cards.id_of(&BEAR));
    assert_eq!(colors(&g, shifter), ColorSet::of(Color::Red), "the kobold copy beneath decides");
    let mark = g.state.mark();
    g.state.effects_retain(|e| e.duration != Duration::EndOfTurn);
    assert_eq!(crate::layers::chars(g.cards, &g.state, &g.log, shifter).card, g.cards.id_of(&BEAR));
    assert_eq!(colors(&g, shifter), ColorSet::of(Color::Blue), "its own color, once the kobold copy has ended");
    assert_eq!(colors(&g, bear), ColorSet::of(Color::Green));
    g.state.revert(mark);
    assert_eq!(colors(&g, shifter), ColorSet::of(Color::Red), "undone, the kobold copy is beneath again");
}

/// A copy of it copies the colors the uncopied color gives it now, as a fixed value (CR 707.9b; the Doppelganger's ruling:
/// "will copy Vesuvan Doppelganger's color"), not the exception.
#[test]
fn a_copy_of_a_copy_with_uncopied_color_copies_its_colors_then() {
    let mut g = game();
    let (shifter, _) = copied_twice(&mut g);
    let values = cx(&mut g).copiable_values(shifter);
    assert!(!values.color_uncopied());
    assert_eq!(values.colors, Some(ColorSet::of(Color::Red)));
    g.state.effects_retain(|e| e.duration != Duration::EndOfTurn);
    let values = cx(&mut g).copiable_values(shifter);
    assert_eq!(values.colors, Some(ColorSet::of(Color::Blue)));
}

/// Gone from the battlefield, it keeps the copiable values it last had (CR 400.7): the effects on it ending after it left
/// change nothing of what a copy of it as it last existed would copy.
#[test]
fn a_departed_copy_keeps_the_uncopied_color_it_last_had() {
    let mut g = game();
    let (shifter, _) = copied_twice(&mut g);
    cantrip_api::Game::exile_all(&mut cx(&mut g), &[shifter]);
    assert!(g.state.obj(shifter).became.is_some());
    g.state.effects_retain(|e| e.affected != shifter);
    let values = cx(&mut g).copiable_values(shifter);
    assert_eq!(values.colors, Some(ColorSet::of(Color::Red)));
}

/// A Room's own colors are those of its unlocked doors' mana costs (CR 709.5): a copy over it that doesn't copy color leaves
/// it those, not the whole card's.
#[test]
fn an_uncopied_color_over_a_room_is_its_unlocked_doors() {
    let mut g = game();
    let room = put(&mut g, &ROOM);
    g.state.update_obj(room, |o| o.marks.doors = DOOR_LEFT);
    let bear = put(&mut g, &BEAR);
    let values = cx(&mut g).copiable_values(bear).uncopied_color();
    cantrip_api::Game::become_copy(&mut cx(&mut g), room, room, values, Duration::Indefinite);
    assert_eq!(colors(&g, room), ColorSet::of(Color::Red));
}
/// Two copy effects that don't copy color, one over the other (the Doppelganger's upkeep trigger twice, CR 113.2c), leave it
/// the colors of the copy beneath both: each is applied over what the one before it left (CR 613.2a, 613.7).
#[test]
fn uncopied_colors_one_over_another_reach_the_copy_beneath_both() {
    let mut g = game();
    let (shifter, bear) = copied_twice(&mut g);
    let values = cx(&mut g).copiable_values(bear).uncopied_color();
    cantrip_api::Game::become_copy(&mut cx(&mut g), shifter, shifter, values, Duration::Indefinite);
    assert_eq!(colors(&g, shifter), ColorSet::of(Color::Red));
    g.state.effects_retain(|e| e.duration != Duration::EndOfTurn);
    assert_eq!(colors(&g, shifter), ColorSet::of(Color::Blue));
}