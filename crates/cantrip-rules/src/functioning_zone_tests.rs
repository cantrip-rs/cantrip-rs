//! Where a triggered ability functions (CR 113.6, `TriggeredDef::functions`): the engine asks it in those zones and nowhere
//! else, so no matcher checks its own zone. "When you cast this spell" functions from the stack with nothing on the card
//! saying so (CR 113.6k, `when_this_spell_is_cast`); an ability that functions on the battlefield doesn't trigger from any
//! other zone, even on a card that is looked at there for another ability; one that functions in a graveyard doesn't
//! trigger from the battlefield.

use crate::{Cx, Game, Setup};
use cantrip_api::*;

/// "When you cast this spell": the spell's own cast.
fn cast_this(_: &dyn Query, source: ObjectId, event: &Event) -> Option<Fired> {
    match event {
        Event::SpellCast { spell, .. } if *spell == source => Fired::PLAIN,
        _ => None,
    }
}

/// "Whenever a player casts a spell", any spell, its own included: nothing here says where the source is.
fn a_spell_cast(_: &dyn Query, _: ObjectId, event: &Event) -> Option<Fired> {
    match event {
        Event::SpellCast { .. } => Fired::PLAIN,
        _ => None,
    }
}

fn gain_two(g: &mut dyn cantrip_api::Game, item: &StackItem) -> Result<(), Suspend> {
    g.gain_life(item.controller, 2);
    Ok(())
}

fn no_return(_: &mut dyn cantrip_api::Game, _: &StackItem) -> Result<(), Suspend> {
    Ok(())
}

/// A cast trigger made the shared way, with no flag, and a battlefield ability that its own cast would match.
static CASTER: CardDef = CardDef {
    name: "Functioning Caster",
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(2),
    toughness: Some(2),
    abilities: &[
        triggered(when_this_spell_is_cast(cast_this), gain_two, "When you cast this spell, you gain 2 life."),
        triggered(when(EventMask::SPELL_CAST, a_spell_cast), gain_two, "Whenever a player casts a spell, you gain 2 life."),
    ],
    ..CardDef::BLANK
};

/// "Whenever a player casts a spell, you may return this card from your graveyard to the battlefield" (CR 113.6m).
static RETURNER: CardDef = CardDef {
    name: "Functioning Returner",
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(2),
    toughness: Some(2),
    abilities: &[triggered(
        when(EventMask::SPELL_CAST, a_spell_cast),
        no_return,
        "Whenever a player casts a spell, you may return this card from your graveyard to the battlefield.",
    )
    .functions_in(IN_A_GRAVEYARD)],
    ..CardDef::BLANK
};

fn cx(game: &mut Game) -> Cx<'_> {
    Cx { g: game, frame: usize::MAX, cursor: 0, mana_source: None, entering: None, lineage: Vec::new(), damage: None, asked_at: None, doer: None, producing: None }
}

fn game() -> (Game, &'static CardTable) {
    let table: &'static CardTable = Box::leak(Box::new(CardTable::new([&CASTER, &RETURNER])));
    (Game::new(table, &Setup { decks: vec![Vec::new(), Vec::new()], ..Default::default() }), table)
}

fn put(game: &mut Game, table: &CardTable, def: &'static CardDef, zone: Zone) -> ObjectId {
    let id = game.state.push_obj(Obj::new(table.id_of(def), PlayerId(0), zone));
    game.state.zone_push(zone, id);
    id
}

/// The abilities of `source` that have triggered, by index.
fn triggered_of(cx: &Cx, source: ObjectId) -> Vec<u8> {
    cx.s().triggers.iter().filter(|t| t.source == source).map(|t| t.ability.index).collect()
}

/// The spell's own cast triggers its "when you cast this spell" on the stack, which the card never declared, and not its
/// "whenever a player casts a spell", which functions only on the battlefield (CR 113.6).
#[test]
fn a_cast_trigger_functions_from_the_stack_and_a_battlefield_ability_does_not() {
    let (mut game, table) = game();
    let spell = put(&mut game, table, &CASTER, Zone::STACK);
    let mut cx = cx(&mut game);
    cx.emit(Event::SpellCast { spell, player: PlayerId(0), nth: 1, options: 0 });
    assert_eq!(triggered_of(&cx, spell), vec![0]);
}

/// Another spell's cast: the Caster on the battlefield triggers its "whenever a player casts a spell"; the Caster in a
/// hand, a graveyard and exile triggers nothing, nor does a Caster spell on the stack - neither of its abilities is its
/// cast's, and the other functions on the battlefield alone.
#[test]
fn a_battlefield_ability_does_not_trigger_from_another_zone() {
    let (mut game, table) = game();
    let on_battlefield = put(&mut game, table, &CASTER, Zone::BATTLEFIELD);
    let elsewhere = [
        put(&mut game, table, &CASTER, Zone::hand(PlayerId(0))),
        put(&mut game, table, &CASTER, Zone::graveyard(PlayerId(0))),
        put(&mut game, table, &CASTER, Zone::exile(PlayerId(0))),
        put(&mut game, table, &CASTER, Zone::STACK),
    ];
    let spell = put(&mut game, table, &RETURNER, Zone::STACK);
    let mut cx = cx(&mut game);
    cx.emit(Event::SpellCast { spell, player: PlayerId(1), nth: 1, options: 0 });
    assert_eq!(triggered_of(&cx, on_battlefield), vec![1]);
    for id in elsewhere {
        assert!(triggered_of(&cx, id).is_empty(), "{:?} triggered from {:?}", id, cx.s().obj(id).zone.kind());
    }
}

/// An ability that returns its card from the graveyard functions only there (CR 113.6m): the Returner in a graveyard
/// triggers on a spell's cast, and on the battlefield or in a hand it doesn't, though its matcher would match.
#[test]
fn a_graveyard_ability_does_not_trigger_from_the_battlefield() {
    let (mut game, table) = game();
    let in_graveyard = put(&mut game, table, &RETURNER, Zone::graveyard(PlayerId(0)));
    let on_battlefield = put(&mut game, table, &RETURNER, Zone::BATTLEFIELD);
    let in_hand = put(&mut game, table, &RETURNER, Zone::hand(PlayerId(0)));
    let spell = put(&mut game, table, &CASTER, Zone::STACK);
    let mut cx = cx(&mut game);
    cx.emit(Event::SpellCast { spell, player: PlayerId(1), nth: 1, options: 0 });
    assert_eq!(triggered_of(&cx, in_graveyard), vec![0]);
    assert!(triggered_of(&cx, on_battlefield).is_empty());
    assert!(triggered_of(&cx, in_hand).is_empty());
}
