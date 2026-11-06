//! The creatures that convoked a spell (CR 702.51c) are in the history: an `Event::Convoked` for each creature tapped to pay
//! for mana in its total cost (CR 702.51a, 601.2h), read with `History::convoked`. A creature tapped for mana while paying
//! convoked nothing; the permanent the spell becomes reads its spell's (CR 608.3); a copy reads the original's (CR 707.10);
//! a creature counts after it has left the battlefield (Knight-Errant of Eos's ruling). Test-only cards.

use crate::{Cx, Game, Setup, ops};
use cantrip_api::*;

const YOU: PlayerId = PlayerId(0);

/// A {3} creature spell with convoke.
static KNIGHT: CardDef = CardDef {
    name: "Convoked Test Knight",
    cost: ManaCost { generic: 3, ..ManaCost::ZERO },
    has_cost: true,
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(4),
    toughness: Some(4),
    pay_with: PayWith { convoke: true, ..PayWith::NONE },
    ..CardDef::BLANK
};
static BEAR: CardDef = CardDef {
    name: "Convoked Test Bear",
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(1),
    toughness: Some(1),
    ..CardDef::BLANK
};
/// A creature with "{T}: Add {C}": tapped for mana, it convokes nothing.
static ELF: CardDef = CardDef {
    name: "Convoked Test Elf",
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(1),
    toughness: Some(1),
    abilities: &[mana_ability(TAP, add(Mana::C), "{T}: Add {C}.")],
    ..CardDef::BLANK
};
static LAND: CardDef = CardDef { name: "Convoked Test Land", has_cost: false, types: TypeSet::of(&[CardType::Land]), ..CardDef::BLANK };
/// A land with "{T}: Add {C}".
static WASTES: CardDef = CardDef { name: "Convoked Test Wastes", abilities: &[mana_ability(TAP, add(Mana::C), "{T}: Add {C}.")], ..LAND };

fn cx(game: &mut Game) -> Cx<'_> {
    Cx { g: game, frame: usize::MAX, cursor: 0, mana_source: None, entering: None, lineage: Vec::new(), damage: None, asked_at: None, doer: None, producing: None }
}

fn put(game: &mut Game, def: &'static CardDef, zone: Zone) -> ObjectId {
    let id = game.state.push_obj(Obj::new(game.cards.id_of(def), YOU, zone));
    game.state.zone_push(zone, id);
    id
}

/// A game at your first main phase with the Knight in your hand and two Bears, an Elf and two Wastes on the battlefield.
fn game() -> (Game, ObjectId, [ObjectId; 5]) {
    let table = Box::leak(Box::new(CardTable::new([&KNIGHT, &BEAR, &ELF, &LAND, &WASTES])));
    let filler = table.id_of(&LAND);
    let mut game = Game::new(table, &Setup { decks: vec![vec![filler; 30], vec![filler; 30]], ..Default::default() });
    let knight = put(&mut game, &KNIGHT, Zone::hand(YOU));
    let creatures = [
        put(&mut game, &BEAR, Zone::BATTLEFIELD),
        put(&mut game, &BEAR, Zone::BATTLEFIELD),
        put(&mut game, &ELF, Zone::BATTLEFIELD),
        put(&mut game, &WASTES, Zone::BATTLEFIELD),
        put(&mut game, &WASTES, Zone::BATTLEFIELD),
    ];
    for _ in 0..150 {
        game.run();
        let r = game.pending().cloned().expect("a question");
        if r.player == YOU && game.state.turn.active == YOU && game.state.turn.step() == Step::PrecombatMain && matches!(r.kind, RequestKind::Priority { .. }) {
            return (game, knight, creatures);
        }
        let answer = match &r.kind {
            RequestKind::Priority { .. } => Answer::Pass,
            RequestKind::DeclareAttackers { .. } => Answer::Attack(Vec::new()),
            RequestKind::DeclareBlockers { .. } => Answer::Block(Vec::new()),
            RequestKind::DiscardToHandSize { count, hand } => Answer::Cards(hand.iter().copied().filter(|&c| c != knight).take(*count as usize).collect()),
            other => panic!("unexpected {other:?}"),
        };
        game.answer(r.id, r.player, answer).unwrap();
    }
    panic!("main phase never reached");
}

/// Answers the pending question with the first option `pick` accepts.
fn act(game: &mut Game, pick: impl Fn(&ActionOption) -> bool) {
    let r = game.pending().cloned().expect("a question");
    let options = match &r.kind {
        RequestKind::Priority { options } | RequestKind::PayCosts { options, .. } => options.clone(),
        other => panic!("unexpected {other:?}"),
    };
    let option = options.iter().find(|o| pick(o)).unwrap_or_else(|| panic!("not offered among {options:?}")).clone();
    game.answer(r.id, r.player, Answer::Act(Action::of(&option))).unwrap();
    game.run();
}

/// Casts the Knight: each of `mana` tapped for {C}, then each of `convoking` tapped for convoke. Returns the spell.
fn cast(game: &mut Game, knight: ObjectId, mana: &[ObjectId], convoking: &[ObjectId]) -> ObjectId {
    act(game, |o| o.kind == ActionKind::CastSpell && o.source == Some(knight));
    for &m in mana {
        act(game, |o| o.kind == ActionKind::ActivateAbility && o.source == Some(m));
    }
    if matches!(game.pending().map(|r| &r.kind), Some(RequestKind::PayCosts { mana_abilities: true, .. })) {
        act(game, |o| o.kind == ActionKind::ConfirmCasting);
    }
    for &c in convoking {
        act(game, |o| o.kind == ActionKind::TapToPay && o.index == PAY_CONVOKE && o.source == Some(c));
    }
    while let Some(RequestKind::PayCosts { .. }) = game.pending().map(|r| &r.kind) {
        act(game, |o| matches!(o.kind, ActionKind::ConfirmCasting | ActionKind::AssignMana));
    }
    let spell = game.state.stack.last().expect("the Knight is on the stack").object;
    assert!(matches!(game.pending().map(|r| &r.kind), Some(RequestKind::Priority { .. })), "the Knight is paid for");
    spell
}

fn convoked(game: &Game, object: ObjectId) -> Vec<ObjectId> {
    game.history().convoked(object).collect()
}

/// CR 702.51c: the two creatures tapped for convoke convoked the spell; the creature tapped for mana as it was paid for did
/// not, though it was tapped too.
#[test]
fn the_creatures_tapped_for_convoke_convoked_the_spell_and_one_tapped_for_mana_did_not() {
    let (mut game, knight, [bear, other, elf, ..]) = game();
    let spell = cast(&mut game, knight, &[elf], &[bear, other]);
    assert!(game.state.obj(elf).tapped, "the Elf was tapped for mana");
    assert_eq!(convoked(&game, spell), [bear, other]);
}

/// Passes priority until the stack is empty.
fn resolve(game: &mut Game) {
    while !game.state.stack.is_empty() {
        let r = game.pending().cloned().expect("a question");
        assert!(matches!(r.kind, RequestKind::Priority { .. }), "unexpected {:?}", r.kind);
        game.answer(r.id, r.player, Answer::Pass).unwrap();
        game.run();
    }
}

/// CR 608.3, 702.51c: the permanent the spell became reads its spell's, and a creature that convoked it still counts once
/// it has died (Knight-Errant of Eos's ruling), named as the object it was.
#[test]
fn the_permanent_reads_its_spells_convokers_and_one_that_died_still_counts() {
    let (mut game, knight, [bear, other, elf, ..]) = game();
    let spell = cast(&mut game, knight, &[elf], &[bear, other]);
    resolve(&mut game);
    let permanent = game.state.obj(spell).became.expect("the Knight resolved");
    assert!(game.state.obj(permanent).zone.is(ZoneKind::Battlefield));
    cantrip_api::Game::destroy(&mut cx(&mut game), bear);
    let dead = game.state.obj(bear).became.expect("the Bear died");
    assert!(game.state.obj(dead).zone.is(ZoneKind::Graveyard));
    assert_eq!(convoked(&game, permanent), [bear, other]);
    assert_eq!(convoked(&game, spell), [bear, other], "the spell, gone from the stack, still has them");
}

/// CR 707.10: a copy of the spell was not cast, but what it says of the objects that paid its costs is the original's: a
/// copy reads the creatures that convoked the original, and so does the token it becomes (CR 707.10f).
#[test]
fn a_copy_of_the_spell_and_its_token_read_the_originals_convokers() {
    let (mut game, knight, [bear, other, elf, wastes, _]) = game();
    let spell = cast(&mut game, knight, &[elf, wastes], &[bear]);
    let copy = ops::copy_spell(&mut cx(&mut game), spell, YOU, false).ok().flatten().expect("the spell is copied");
    assert_ne!(copy, spell);
    assert_eq!(convoked(&game, copy), [bear]);
    let at = game.state.stack.iter().position(|i| i.object == copy).expect("the copy is on the stack");
    let item = game.state.stack[at];
    ops::resolve_copy_as_token(&mut cx(&mut game), &item, at).expect("the copy resolves");
    let token = *game.state.battlefield().iter().find(|&&id| game.state.obj(id).token).expect("the copy became a token");
    assert_eq!(convoked(&game, token), [bear]);
    assert!(!convoked(&game, spell).contains(&other), "an untapped creature convoked nothing");
}

/// No creature convoked a spell paid for without convoke, nor a permanent that was never a spell.
#[test]
fn nothing_convoked_a_spell_paid_with_mana_or_a_permanent_never_cast() {
    let (mut game, knight, [bear, _, elf, wastes, other_wastes]) = game();
    let spell = cast(&mut game, knight, &[elf, wastes, other_wastes], &[]);
    assert!(!game.state.obj(bear).tapped);
    assert_eq!(convoked(&game, spell), []);
    assert_eq!(convoked(&game, bear), []);
}
