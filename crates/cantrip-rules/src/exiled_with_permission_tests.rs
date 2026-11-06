//! "Until end of turn, you may play [a card | cards] exiled with [this]" (`Game::allow_play_exiled_with`,
//! `Playable::ExiledWith`; CR 607.2a, 611.2a, 611.2c): Raphael, Most Attitude's and Lobelia, Defender of Bag End's kind of
//! permission, over test cards. It is the ability's controller's alone; it covers the cards the object's linked ability
//! exiles after it began, and none another object exiled; it goes on when the object leaves, and a returned object is a new
//! one whose cards it never covers (CR 400.7); it ends in the cleanup step (CR 514.2); and "a card" is used up by the first
//! card played by it.

use crate::actions::{permissions_for, priority_options};
use crate::{Cx, Game, Setup, ops};
use cantrip_api::*;

fn nothing(_: &mut dyn cantrip_api::Game, _: &StackItem) -> Result<(), Suspend> {
    Ok(())
}

static TURTLE: CardDef = CardDef {
    name: "Exiled With Test Turtle",
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(2),
    toughness: Some(2),
    ..CardDef::BLANK
};
static OTHER: CardDef = CardDef { name: "Exiled With Test Other", ..TURTLE };
static LAND: CardDef = CardDef { name: "Exiled With Test Land", has_cost: false, types: TypeSet::of(&[CardType::Land]), ..CardDef::BLANK };
static SPELL: CardDef =
    CardDef { name: "Exiled With Test Spell", types: TypeSet::of(&[CardType::Sorcery]), spell: spell(&[], nothing), ..CardDef::BLANK };

const YOU: PlayerId = PlayerId(0);
const THEM: PlayerId = PlayerId(1);

struct Table {
    game: Game,
    turtle: ObjectId,
    other: ObjectId,
    /// Two lands and a sorcery of yours in your graveyard, to be exiled.
    lands: [ObjectId; 2],
    spell: ObjectId,
}

fn table() -> Table {
    let table: &'static CardTable = Box::leak(Box::new(CardTable::new([&TURTLE, &OTHER, &LAND, &SPELL])));
    let filler = table.id_of(&LAND);
    let mut game = Game::new(table, &Setup { decks: vec![vec![filler; 30], vec![filler; 30]], ..Default::default() });
    let put = |game: &mut Game, def: &'static CardDef, zone: Zone| {
        let id = game.state.push_obj(Obj::new(table.id_of(def), YOU, zone));
        game.state.zone_push(zone, id);
        id
    };
    let turtle = put(&mut game, &TURTLE, Zone::BATTLEFIELD);
    let other = put(&mut game, &OTHER, Zone::BATTLEFIELD);
    let lands = [put(&mut game, &LAND, Zone::graveyard(YOU)), put(&mut game, &LAND, Zone::graveyard(YOU))];
    let spell = put(&mut game, &SPELL, Zone::graveyard(YOU));
    Table { game, turtle, other, lands, spell }
}

/// Your precombat main phase, set by hand, for asking what may be played.
fn in_main_phase(game: &mut Game) {
    game.state.update_turn(|t| {
        t.active = YOU;
        t.priority = YOU;
        t.set_step(Step::PrecombatMain);
    });
}

fn cx(game: &mut Game) -> Cx<'_> {
    Cx { g: game, frame: usize::MAX, cursor: 0, mana_source: None, entering: None, lineage: Vec::new(), damage: None, asked_at: None, doer: None, producing: None }
}

/// An ability of `object`'s own text, as linked abilities know it (CR 607.1).
fn linked(cx: &Cx, object: ObjectId) -> Linked {
    Linked { object, text: cx.s().obj(object).card, acquired: Acquired::OWN }
}

fn exile_with(cx: &mut Cx, card: ObjectId, by: ObjectId) -> ObjectId {
    let by = linked(cx, by);
    ops::exile_with(cx, &[card], by, false)[0]
}

fn allow(cx: &mut Cx, by: ObjectId, player: PlayerId, one: bool) {
    let by = linked(cx, by);
    cantrip_api::Game::allow_play_exiled_with(cx, by, player, PlayGrant { lands: true, expiry: Expiry::EndOfTurn, ..PlayGrant::CAST }, one);
}

fn land_playable(cx: &mut Cx, player: PlayerId, land: ObjectId) -> bool {
    priority_options(cx, player).iter().any(|o| o.kind == ActionKind::PlayLand && o.source == Some(land))
}

fn castable(cx: &Cx, player: PlayerId, card: ObjectId) -> bool {
    permissions_for(cx, card, player).next().is_some()
}

#[test]
fn it_covers_cards_exiled_with_the_object_later_and_none_another_object_exiled() {
    let Table { mut game, turtle, other, lands, spell } = table();
    in_main_phase(&mut game);
    let mut cx = cx(&mut game);
    // Granted with nothing exiled yet: a rule of the game, it reaches the cards exiled after it began (CR 611.2c).
    allow(&mut cx, turtle, YOU, false);
    let mine = exile_with(&mut cx, lands[0], turtle);
    let spell = exile_with(&mut cx, spell, turtle);
    // Exiled by another object, or by nothing: not "exiled with" the turtle (CR 607.2a).
    let others = exile_with(&mut cx, lands[1], other);
    assert!(land_playable(&mut cx, YOU, mine));
    assert!(castable(&cx, YOU, spell));
    assert!(!land_playable(&mut cx, YOU, others));
    assert!(!castable(&cx, YOU, others));
    let plain = ops::move_to(&mut cx, others, Zone::exile(YOU));
    assert!(cx.s().obj(plain).zone.is(ZoneKind::Exile) && cx.s().obj(plain).exiled.is_none());
    assert!(!castable(&cx, YOU, plain));
    // The ability's controller's alone (CR 611.2a): the opponent may play none of them.
    assert!(!castable(&cx, THEM, spell));
    assert!(!castable(&cx, THEM, mine));
    // A card that leaves exile and comes back is a new object, exiled with nothing (CR 400.7).
    let gone = ops::move_to(&mut cx, mine, Zone::graveyard(YOU));
    let back = ops::move_to(&mut cx, gone, Zone::exile(YOU));
    assert!(!land_playable(&mut cx, YOU, back));
}

#[test]
fn it_outlives_the_object_and_never_covers_what_the_object_exiles_once_it_has_returned() {
    let Table { mut game, turtle, lands, spell, .. } = table();
    in_main_phase(&mut game);
    let mut cx = cx(&mut game);
    allow(&mut cx, turtle, YOU, false);
    let first = exile_with(&mut cx, lands[0], turtle);
    // It leaves the battlefield: an effect of a resolved ability, the permission goes on (CR 611.2a).
    let dead = ops::move_to(&mut cx, turtle, Zone::graveyard(YOU));
    crate::settle::state_based_actions_alone(&mut cx).unwrap();
    assert!(land_playable(&mut cx, YOU, first));
    // Back on the battlefield it is a new object (CR 400.7): what it exiles now is not exiled with the one the permission
    // names (Raphael, Most Attitude's ruling), while the old one's card still is.
    let returned = ops::move_to(&mut cx, dead, Zone::BATTLEFIELD);
    let later = exile_with(&mut cx, spell, returned);
    assert!(!castable(&cx, YOU, later));
    assert!(land_playable(&mut cx, YOU, first));
    let second = exile_with(&mut cx, lands[1], returned);
    assert!(!land_playable(&mut cx, YOU, second));
}

#[test]
fn it_ends_in_the_cleanup_step() {
    let Table { mut game, turtle, lands, .. } = table();
    in_main_phase(&mut game);
    let mut cx = cx(&mut game);
    allow(&mut cx, turtle, YOU, false);
    let exiled = exile_with(&mut cx, lands[0], turtle);
    assert!(land_playable(&mut cx, YOU, exiled));
    crate::turn::cleanup(&mut cx).unwrap();
    assert!(cx.s().permissions.is_empty());
    in_main_phase(cx.g);
    assert!(!land_playable(&mut cx, YOU, exiled));
}

/// Runs the game to your first precombat main phase with priority, after `setup` and with the turtle's land and sorcery
/// exiled with it; plays the land by the permission; and says whether the sorcery may still be cast by one.
fn after_playing_one(setup: impl Fn(&mut Cx, ObjectId)) -> bool {
    let Table { mut game, turtle, lands, spell, .. } = table();
    let (land, spell) = {
        let mut cx = cx(&mut game);
        let land = exile_with(&mut cx, lands[0], turtle);
        let spell = exile_with(&mut cx, spell, turtle);
        setup(&mut cx, turtle);
        (land, spell)
    };
    let mut played = false;
    for _ in 0..200 {
        game.run();
        let r = game.pending().cloned().expect("a question");
        if r.player == YOU && game.state.turn.active == YOU && game.state.turn.step() == Step::PrecombatMain {
            let RequestKind::Priority { options } = &r.kind else { panic!("{:?}", r.kind) };
            if !played {
                let option = options.iter().find(|o| o.kind == ActionKind::PlayLand && o.source == Some(land)).expect("the exiled land offered");
                game.answer(r.id, r.player, Answer::Act(Action::of(option))).unwrap();
                played = true;
                continue;
            }
            assert!(game.state.obj(game.state.obj(land).became.unwrap()).zone.is(ZoneKind::Battlefield));
            return options.iter().any(|o| o.kind == ActionKind::CastSpell && o.source == Some(spell));
        }
        let answer = match &r.kind {
            RequestKind::Priority { .. } => Answer::Pass,
            RequestKind::DeclareAttackers { .. } => Answer::Attack(Vec::new()),
            RequestKind::DeclareBlockers { .. } => Answer::Block(Vec::new()),
            RequestKind::DiscardToHandSize { count, hand } => Answer::Cards(hand.iter().copied().take(*count as usize).collect()),
            other => panic!("unexpected {other:?}"),
        };
        game.answer(r.id, r.player, answer).unwrap();
    }
    panic!("your main phase never came");
}

#[test]
fn a_card_is_used_up_by_the_first_card_played_by_it() {
    // "You may play a card exiled with [it]": the land played, the sorcery may not be cast too.
    assert!(!after_playing_one(|cx, turtle| allow(cx, turtle, YOU, true)));
    // "Cards": both.
    assert!(after_playing_one(|cx, turtle| allow(cx, turtle, YOU, false)));
    // Two triggers' permissions, each for a card: one is used, the other lets the sorcery be cast.
    assert!(after_playing_one(|cx, turtle| {
        allow(cx, turtle, YOU, true);
        allow(cx, turtle, YOU, true);
    }));
}

/// Casting a spell by "a card" uses it up as it is cast (CR 601.2a); "cards" goes on.
#[test]
fn a_spell_cast_by_a_card_uses_it_up() {
    for one in [true, false] {
        let Table { mut game, turtle, spell, .. } = table();
        let spell = {
            let mut cx = cx(&mut game);
            let spell = exile_with(&mut cx, spell, turtle);
            allow(&mut cx, turtle, YOU, one);
            spell
        };
        let mut cast = false;
        for _ in 0..200 {
            game.run();
            let r = game.pending().cloned().expect("a question");
            if r.player == YOU && game.state.turn.active == YOU && game.state.turn.step() == Step::PrecombatMain {
                let RequestKind::Priority { options } = &r.kind else { panic!("{:?}", r.kind) };
                let option = options.iter().find(|o| o.kind == ActionKind::CastSpell && o.source == Some(spell)).expect("the exiled sorcery offered");
                game.answer(r.id, r.player, Answer::Act(Action::of(option))).unwrap();
                game.run();
                cast = true;
                break;
            }
            let answer = match &r.kind {
                RequestKind::Priority { .. } => Answer::Pass,
                RequestKind::DeclareAttackers { .. } => Answer::Attack(Vec::new()),
                RequestKind::DeclareBlockers { .. } => Answer::Block(Vec::new()),
                other => panic!("unexpected {other:?}"),
            };
            game.answer(r.id, r.player, answer).unwrap();
        }
        assert!(cast);
        assert!(game.state.obj(spell).became.is_some_and(|s| game.state.obj(s).zone.is(ZoneKind::Stack)));
        assert_eq!(game.state.permissions.is_empty(), one);
    }
}
