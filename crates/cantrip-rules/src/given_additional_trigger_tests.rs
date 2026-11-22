//! A rule an object is given that makes abilities trigger an additional time (`AbilityDef::TriggersAgain`, CR 603.2d, 613.1f),
//! asked of the objects that have it as the printed `CardDef::additional_triggers` entries are, over test cards: the holder's
//! own abilities trigger again and another creature's do not; the rule is its holder's only while it has it.

use crate::{Cx, Game, Setup};
use cantrip_api::Game as _;
use cantrip_api::*;

const YOU: PlayerId = PlayerId(0);

fn fires(_: &dyn Query, _: ObjectId, event: &Event) -> Option<Fired> {
    if matches!(event, Event::LifeChange { .. }) { Fired::PLAIN } else { None }
}

/// "If a life total changing causes a triggered ability of this creature to trigger, that ability triggers an additional time."
fn own_abilities_again(_: &dyn Query, asker: &Asker, event: &Event, triggering: &Triggering) -> bool {
    matches!(event, Event::LifeChange { .. }) && triggering.permanent && triggering.source == asker.source
}

/// A creature that lists the rule to give itself, with an ability that triggers on a life total changing.
static HOLDER: CardDef = CardDef {
    name: "Given Trigger Test Holder",
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(1),
    toughness: Some(1),
    abilities: &[triggered(when(EventMask::LIFE_CHANGE, fires), |_, _| Ok(()), "Whenever a life total changes, nothing.")],
    delayed: &[granted_additional_trigger(own_abilities_again, "If a life total changing causes a triggered ability of this creature to trigger, that ability triggers an additional time.")],
    ..CardDef::BLANK
};

/// Another creature with the same ability, which the rule is not about.
static BEAR: CardDef = CardDef { name: "Given Trigger Test Bear", delayed: &[], ..HOLDER };

fn game() -> Game {
    let table: &'static CardTable = Box::leak(Box::new(CardTable::new([&HOLDER, &BEAR])));
    Game::new(table, &Setup { decks: vec![Vec::new(), Vec::new()], ..Default::default() })
}

fn enter(game: &mut Game, def: &'static CardDef) -> ObjectId {
    let id = game.state.push_obj(Obj::new(game.cards.id_of(def), YOU, Zone::BATTLEFIELD));
    game.state.zone_push(Zone::BATTLEFIELD, id);
    id
}

fn cx(game: &mut Game) -> Cx<'_> {
    Cx { g: game, frame: usize::MAX, cursor: 0, mana_source: None, entering: None, lineage: Vec::new(), damage: None, asked_at: None, doer: None, producing: None, reached: None }
}

/// How many times each of two permanents' abilities is waiting to be put on the stack after a life total changed.
fn triggers_after_a_life_change(game: &mut Game, holder: ObjectId, bear: ObjectId) -> (usize, usize) {
    game.state.triggers.clear();
    cx(game).emit(Event::LifeChange { player: YOU, delta: -1 });
    let of = |id| game.state.triggers.iter().filter(|t| t.source == id).count();
    (of(holder), of(bear))
}

/// The rule given to the holder (an effect gives it the card's listed ability) makes its own ability trigger twice and leaves
/// the other creature's at one, given twice it triggers three times; before it was given, and after the holder loses all abilities, each triggers once.
#[test]
fn a_given_rule_doubles_its_holders_own_abilities_only_while_it_has_it() {
    let mut game = game();
    let table = game.cards;
    let holder = enter(&mut game, &HOLDER);
    let bear = enter(&mut game, &BEAR);
    assert_eq!(triggers_after_a_life_change(&mut game, holder, bear), (1, 1), "nothing given yet");

    let timestamp = game.state.timestamp();
    let given = AbilityRef::delayed(table.id_of(&HOLDER), 0);
    game.state.effect_push(ContinuousEffect { source: holder, controller: YOU, timestamp, affected: holder, kind: EffectKind::AddAbility(given), duration: Duration::Indefinite, words: Words::NONE });
    assert_eq!(triggers_after_a_life_change(&mut game, holder, bear), (2, 1), "the holder's ability triggers an additional time, the bear's does not");

    let timestamp = game.state.timestamp();
    game.state.effect_push(ContinuousEffect { source: holder, controller: YOU, timestamp, affected: holder, kind: EffectKind::AddAbility(given), duration: Duration::Indefinite, words: Words::NONE });
    assert_eq!(triggers_after_a_life_change(&mut game, holder, bear), (3, 1), "given twice, it triggers two additional times (CR 603.2d)");

    cx(&mut game).add_effect(holder, holder, EffectKind::LoseAllAbilities, Duration::Indefinite);
    assert_eq!(triggers_after_a_life_change(&mut game, holder, bear), (0, 1), "a holder that lost all abilities has neither its own nor the given");
}
