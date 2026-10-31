//! A play rule an effect gives a permanent (`AbilityDef::PlayRule`, CR 613.1f) has, as a continuous effect, the later of the
//! permanent's timestamp and that of the effect that gave it (CR 613.7a): a given "You have no maximum hand size" (Nerd
//! Rage's kind) is judged against the other maximum-hand-size effects in that order (CR 613.11, 613.7). Over test cards.

use crate::actions::maximum_hand_size;
use crate::{Cx, Game, Setup};
use cantrip_api::*;

const YOU: PlayerId = PlayerId(0);

/// A creature that lists "You have no maximum hand size" to give itself.
static HOLDER: CardDef = CardDef {
    name: "Given Play Rule Test Holder",
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(1),
    toughness: Some(1),
    delayed: &[granted_play_rule(PlayRule::MaximumHandSize(None), "You have no maximum hand size.")],
    ..CardDef::BLANK
};

/// "Other creatures you control have 'You have no maximum hand size'", a static ability that gives its card's listed rule.
fn others_you_control(q: &dyn LayerQuery, source: ObjectId, object: ObjectId, chars: &Chars) -> bool {
    object != source && chars.is(CardType::Creature) && q.state().obj(object).controller == q.state().obj(source).controller
}

fn give_no_maximum(q: &dyn LayerQuery, source: ObjectId, chars: &mut Chars) {
    let given = AbilityRef::delayed(q.state().obj(source).card, 0);
    if let Some(slot) = chars.abilities.iter().position(|a| a.is_none()) {
        chars.abilities[slot] = Some(given);
    }
}

static GIVER: CardDef = CardDef {
    name: "Given Play Rule Test Giver",
    types: TypeSet::of(&[CardType::Enchantment]),
    statics: &[static_ability(Layer::Ability, others_you_control, give_no_maximum, "Other creatures you control have \"You have no maximum hand size.\"")],
    delayed: &[granted_play_rule(PlayRule::MaximumHandSize(None), "You have no maximum hand size.")],
    ..CardDef::BLANK
};

/// A plain creature to be given the rule.
static BEAR: CardDef = CardDef { name: "Given Play Rule Test Bear", types: TypeSet::of(&[CardType::Creature]), power: Some(2), toughness: Some(2), ..CardDef::BLANK };

/// A card `YOU` own and control on the battlefield, entered now.
fn enter(game: &mut Game, def: &'static CardDef) -> ObjectId {
    let id = game.state.push_obj(Obj::new(game.cards.id_of(def), YOU, Zone::BATTLEFIELD));
    let entered = game.state.timestamp();
    game.state.update_obj(id, |o| o.timestamp = entered);
    game.state.zone_push(Zone::BATTLEFIELD, id);
    // A permanent with a static ability entered, as the engine marks it.
    if !def.statics.is_empty() {
        game.state.see_statics();
    }
    id
}

fn cx(game: &mut Game) -> Cx<'_> {
    Cx { g: game, frame: usize::MAX, cursor: 0, mana_source: None, entering: None, lineage: Vec::new(), asked_at: None, doer: None, producing: None }
}

fn game() -> Game {
    let table: &'static CardTable = Box::leak(Box::new(CardTable::new([&HOLDER, &GIVER, &BEAR])));
    Game::new(table, &Setup { decks: vec![Vec::new(), Vec::new()], ..Default::default() })
}

/// An effect of `holder`'s, made now, about `affected`.
fn effect(game: &mut Game, holder: ObjectId, affected: ObjectId, kind: EffectKind) {
    let timestamp = game.state.timestamp();
    game.state.effect_push(ContinuousEffect { source: holder, controller: YOU, timestamp, affected, kind, duration: Duration::Indefinite });
}

/// The holder on the battlefield, entered first; then "your maximum hand size is five" from a resolved spell; then the holder
/// given "You have no maximum hand size". The given rule's timestamp is the grant's, later than the five's, so you have no
/// maximum - read at the holder's own timestamp it would be the oldest, and the five would win. A maximum of four made after
/// the grant is newer still, and wins.
#[test]
fn a_given_maximum_hand_size_has_the_timestamp_of_the_effect_that_gave_it() {
    let mut game = game();
    let table = game.cards;
    let holder = enter(&mut game, &HOLDER);
    assert_eq!(maximum_hand_size(&cx(&mut game), YOU), Some(7), "nothing given yet");

    effect(&mut game, holder, holder, EffectKind::MaximumHandSize(Some(5)));
    assert_eq!(maximum_hand_size(&cx(&mut game), YOU), Some(5));

    effect(&mut game, holder, holder, EffectKind::AddAbility(AbilityRef::delayed(table.id_of(&HOLDER), 0)));
    assert_eq!(maximum_hand_size(&cx(&mut game), YOU), None, "the given rule is as new as its grant, newer than the five (CR 613.7a)");

    effect(&mut game, holder, holder, EffectKind::MaximumHandSize(Some(4)));
    assert_eq!(maximum_hand_size(&cx(&mut game), YOU), Some(4), "a maximum made after the grant is newer");
}

/// The same order with a static ability as the giver: the creature enters, then "your maximum hand size is five", then the
/// permanent whose static ability gives the creature "You have no maximum hand size". The given rule's timestamp is the
/// giver's (CR 613.7a), so you have no maximum.
#[test]
fn a_maximum_hand_size_a_static_ability_gives_has_its_givers_timestamp() {
    let mut game = game();
    let bear = enter(&mut game, &BEAR);
    effect(&mut game, bear, bear, EffectKind::MaximumHandSize(Some(5)));
    assert_eq!(maximum_hand_size(&cx(&mut game), YOU), Some(5));
    enter(&mut game, &GIVER);
    assert_eq!(maximum_hand_size(&cx(&mut game), YOU), None, "the given rule is as new as its giver, newer than the five (CR 613.7a)");
    effect(&mut game, bear, bear, EffectKind::MaximumHandSize(Some(4)));
    assert_eq!(maximum_hand_size(&cx(&mut game), YOU), Some(4), "a maximum made after the giver entered is newer");
}
