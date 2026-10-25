//! CR 506.4, 506.4c, 506.4e: a planeswalker or battle being attacked is removed from combat the moment its controller or
//! protector changes or it stops being one, for good - the creatures attacking it go on attacking nothing, and assign no
//! combat damage, though it be one again (or theirs again) before combat damage. CR 506.4, 506.4d: so is an attacking or
//! blocking creature that stops being a creature. Over test cards, through the engine's own operations: a
//! control-changing effect, the state-based action that gives a battle a new protector, type-changing effects, the source
//! of an animating effect leaving.

use super::*;
use crate::Game as RulesGame;

const YOU: PlayerId = PlayerId(0);
const THEM: PlayerId = PlayerId(1);
const OTHER: PlayerId = PlayerId(2);

static ATTACKER: CardDef = CardDef {
    name: "Removed combat attacker probe",
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(3),
    toughness: Some(3),
    ..CardDef::BLANK
};
static WALKER: CardDef = CardDef { name: "Removed combat walker probe", types: TypeSet::of(&[CardType::Planeswalker]), loyalty: Some(10), ..CardDef::BLANK };
static BATTLE: CardDef = CardDef { name: "Removed combat battle probe", types: TypeSet::of(&[CardType::Battle]), defense: Some(10), ..CardDef::BLANK };
/// A permanent that is both a planeswalker and a battle (CR 506.4e), protected by its controller (no Siege, CR 310.9a).
static BOTH: CardDef = CardDef {
    name: "Removed combat walker battle probe",
    types: TypeSet::of(&[CardType::Planeswalker, CardType::Battle]),
    loyalty: Some(10),
    defense: Some(10),
    ..CardDef::BLANK
};
/// The same, a Siege: protected by an opponent of its controller (CR 310.9a).
static BOTH_SIEGE: CardDef = CardDef { name: "Removed combat walker siege probe", subtypes: SubtypeSet::of(&[Subtype::Siege]), ..BOTH };

static ARTIFACT: CardDef = CardDef { name: "Removed combat artifact probe", types: TypeSet::of(&[CardType::Artifact]), ..CardDef::BLANK };
/// What animates an artifact "for as long as" it is on the battlefield: Ensoul Artifact, March of the Machines.
static ANIMATOR: CardDef = CardDef { name: "Removed combat animator probe", types: TypeSet::of(&[CardType::Enchantment]), ..CardDef::BLANK };
static LIFELINKER: CardDef = CardDef { name: "Removed combat lifelink probe", keywords: Keywords::of(&[Keyword::Lifelink]), ..ATTACKER };
/// A planeswalker that is a creature too (Gideon, Kaito during his controller's turn).
static WALKER_CREATURE: CardDef = CardDef {
    name: "Removed combat walker creature probe",
    types: TypeSet::of(&[CardType::Planeswalker, CardType::Creature]),
    power: Some(2),
    toughness: Some(2),
    loyalty: Some(10),
    ..CardDef::BLANK
};

/// The attacker as the combat state has it, attacking `target` as `attacked_as` with `defending` its defending player.
fn attacking(id: ObjectId, target: Target, defending: PlayerId, attacked_as: TypeSet) -> Attacker {
    Attacker::new(id, target, defending, attacked_as)
}

/// A game of `players` in YOU's declare blockers step: YOU's creature attacks `def`, a permanent `controller` controls and
/// `protector` protects, whose defending player is `defending`.
fn game(players: usize, def: &'static CardDef, controller: PlayerId, protector: Option<PlayerId>, defending: PlayerId) -> (RulesGame, ObjectId, ObjectId) {
    let cards = Box::leak(Box::new(CardTable::new([&ATTACKER, &WALKER, &BATTLE, &BOTH, &BOTH_SIEGE, &ARTIFACT, &ANIMATOR, &LIFELINKER, &WALKER_CREATURE])));
    let mut game = RulesGame::new(cards, &crate::Setup { decks: vec![Vec::new(); players], ..Default::default() });
    // Nobody loses to the state-based actions for the opening draw from an empty library.
    for p in 0..players {
        game.state.update_player(PlayerId(p as u8), |p| p.drew_from_empty = false);
    }
    game.state.update_turn(|t| {
        t.number = 1;
        t.active = YOU;
        t.set_step(Step::DeclareBlockers);
        t.attacked = true;
    });
    let attacker = put(&mut game, &ATTACKER, YOU);
    let attacked = put(&mut game, def, controller);
    game.state.update_obj(attacked, |o| o.marks.protector = protector);
    game.state.set_attackers(vec![attacking(attacker, Target::Object(attacked), defending, def.types)]);
    (game, attacker, attacked)
}

fn put(game: &mut RulesGame, def: &'static CardDef, controller: PlayerId) -> ObjectId {
    let id = game.state.push_obj(Obj::new(game.cards.id_of(def), controller, Zone::BATTLEFIELD));
    let timestamp = game.state.timestamp();
    game.state.update_obj(id, |o| o.timestamp = timestamp);
    game.state.zone_push(Zone::BATTLEFIELD, id);
    id
}

fn cx(game: &mut RulesGame) -> Cx<'_> {
    Cx { g: game, frame: usize::MAX, cursor: 0, mana_source: None, entering: None, lineage: Vec::new(), asked_at: None, doer: None, producing: None }
}

/// An effect from the attacked permanent itself on it: what a resolved spell's would be.
fn effect(cx: &mut Cx, on: ObjectId, kind: EffectKind) {
    cantrip_api::Game::add_effect(cx, on, on, kind, Duration::EndOfTurn);
}

/// The combat damage step: the combat damage dealt to `attacked`.
fn damage_to(cx: &mut Cx, attacked: ObjectId) -> u32 {
    cx.st().update_turn(|t| t.set_step(Step::CombatDamage));
    combat_damage(cx).unwrap();
    cx.g.log
        .iter()
        .filter_map(|r| match r.event {
            Event::Damage { combat: true, target: Target::Object(id), amount, .. } if id == attacked => Some(u32::from(amount)),
            _ => None,
        })
        .sum()
}

/// Still an attacking creature, attacking nothing, its defending player the one it had (CR 506.4c, 508.5).
fn attacks_nothing(cx: &Cx, attacker: ObjectId, defending: PlayerId) {
    assert_eq!(cx.s().attackers.len(), 1, "still an attacking creature");
    assert_eq!(cx.s().attackers[0].id, attacker);
    assert_eq!(cx.s().attackers[0].target, Target::None, "attacking nothing");
    assert_eq!(cx.s().attackers[0].defending, defending, "its defending player is the one it had");
}

/// Their planeswalker is attacked; during the declare blockers step you gain control of it. It is removed from combat
/// (CR 506.4): your creature, unblocked, deals it no combat damage (CR 506.4c, 510.1b).
#[test]
fn an_attacked_planeswalker_whose_controller_changes_is_dealt_no_combat_damage() {
    let (mut game, attacker, walker) = game(2, &WALKER, THEM, None, THEM);
    let mut cx = cx(&mut game);
    effect(&mut cx, walker, EffectKind::Control(YOU));
    assert_eq!(cx.s().obj(walker).controller, YOU);
    attacks_nothing(&cx, attacker, THEM);
    assert_eq!(damage_to(&mut cx, walker), 0);
}

/// Their battle (protected by them) is attacked; it becomes a Siege, so they can no longer protect it, and the
/// state-based actions give it a new protector - you (CR 310.11, 704.5x). Its protector changed: it is removed from
/// combat (CR 506.4), and dealt no combat damage.
#[test]
fn an_attacked_battle_whose_protector_changes_is_dealt_no_combat_damage() {
    let (mut game, attacker, battle) = game(2, &BATTLE, THEM, Some(THEM), THEM);
    let mut cx = cx(&mut game);
    // Defense counters, so the state-based actions keep it on the battlefield (CR 704.5v).
    cantrip_api::Game::add_counters(&mut cx, battle, CounterKind::Defense, 10);
    effect(&mut cx, battle, EffectKind::AddSubtype(Subtype::Siege));
    crate::settle::state_based_actions(&mut cx).unwrap();
    assert_eq!(cx.s().obj(battle).marks.protector, Some(YOU), "the state-based actions chose a new protector");
    attacks_nothing(&cx, attacker, THEM);
    assert_eq!(damage_to(&mut cx, battle), 0);
}

/// Their planeswalker is attacked; an effect makes it a creature and nothing else, and a later one makes it a
/// planeswalker again, before combat damage. It stopped being a planeswalker, so it was removed from combat for good
/// (CR 506.4), and is dealt no combat damage.
#[test]
fn an_attacked_planeswalker_that_stops_being_one_and_becomes_one_again_is_dealt_no_combat_damage() {
    let (mut game, attacker, walker) = game(2, &WALKER, THEM, None, THEM);
    let mut cx = cx(&mut game);
    effect(&mut cx, walker, EffectKind::SetTypes(TypeSet::of(&[CardType::Creature])));
    effect(&mut cx, walker, EffectKind::AddTypes(TypeSet::of(&[CardType::Planeswalker])));
    assert!(cx.chars(walker).is(CardType::Planeswalker), "a planeswalker again");
    attacks_nothing(&cx, attacker, THEM);
    assert_eq!(damage_to(&mut cx, walker), 0);
}

/// The same for a battle: one that stops being a battle and becomes one again is not attacked any more.
#[test]
fn an_attacked_battle_that_stops_being_one_and_becomes_one_again_is_dealt_no_combat_damage() {
    let (mut game, attacker, battle) = game(2, &BATTLE, THEM, Some(THEM), THEM);
    let mut cx = cx(&mut game);
    effect(&mut cx, battle, EffectKind::SetTypes(TypeSet::of(&[CardType::Artifact])));
    effect(&mut cx, battle, EffectKind::AddTypes(TypeSet::of(&[CardType::Battle])));
    assert!(cx.chars(battle).is(CardType::Battle), "a battle again");
    attacks_nothing(&cx, attacker, THEM);
    assert_eq!(damage_to(&mut cx, battle), 0);
}

/// CR 506.4e: a permanent being attacked that is both a planeswalker and a battle stays in combat while it is either -
/// a battle and no longer a planeswalker; a planeswalker and no longer a battle, controlled by its protector - and is
/// dealt the combat damage; once it is neither, it is removed, and is dealt none.
#[test]
fn a_planeswalker_battle_stays_attacked_while_it_is_either() {
    for (types, stays) in [
        (TypeSet::of(&[CardType::Battle]), true),
        (TypeSet::of(&[CardType::Planeswalker]), true),
        (TypeSet::of(&[CardType::Artifact]), false),
    ] {
        let (mut game, attacker, both) = game(2, &BOTH, THEM, Some(THEM), THEM);
        let mut cx = cx(&mut game);
        effect(&mut cx, both, EffectKind::SetTypes(types));
        if stays {
            assert_eq!(cx.s().attackers[0].target, Target::Object(both), "still attacked as {types:?}");
            assert_eq!(damage_to(&mut cx, both), 3);
        } else {
            attacks_nothing(&cx, attacker, THEM);
            assert_eq!(damage_to(&mut cx, both), 0);
        }
    }
}

/// CR 506.4e: a Siege that is also a planeswalker, controlled by one opponent and protected by the player attacked, stops
/// being a battle: a planeswalker not controlled by its protector, it is removed from combat, and is dealt no combat damage.
#[test]
fn a_planeswalker_battle_that_stops_being_a_battle_not_controlled_by_its_protector_is_removed() {
    let (mut game, attacker, both) = game(3, &BOTH_SIEGE, OTHER, Some(THEM), THEM);
    let mut cx = cx(&mut game);
    effect(&mut cx, both, EffectKind::SetTypes(TypeSet::of(&[CardType::Planeswalker])));
    attacks_nothing(&cx, attacker, THEM);
    assert_eq!(damage_to(&mut cx, both), 0);
}

/// A planeswalker being attacked that is never changed is dealt the combat damage, and stays attacked through the
/// checks: the removal is not over-eager.
#[test]
fn an_attacked_planeswalker_left_alone_is_dealt_the_combat_damage() {
    let (mut game, _, walker) = game(2, &WALKER, THEM, None, THEM);
    let mut cx = cx(&mut game);
    // Loyalty counters, so the state-based actions keep it on the battlefield (CR 704.5i).
    cantrip_api::Game::add_counters(&mut cx, walker, CounterKind::Loyalty, 10);
    crate::settle::state_based_actions(&mut cx).unwrap();
    crate::ops::settle_lasting_and_control(&mut cx);
    assert_eq!(cx.s().attackers[0].target, Target::Object(walker));
    assert_eq!(damage_to(&mut cx, walker), 3);
}

/// A battle protected by its controller (no Siege), being attacked, becomes a planeswalker and stops being a battle in one
/// change. It was never both, so CR 506.4e is not about it: it stopped being the battle it was attacked as, and CR 506.4
/// removes it from combat.
#[test]
fn an_attacked_battle_that_becomes_only_a_planeswalker_is_dealt_no_combat_damage() {
    let (mut game, attacker, battle) = game(2, &BATTLE, THEM, Some(THEM), THEM);
    let mut cx = cx(&mut game);
    effect(&mut cx, battle, EffectKind::SetTypes(TypeSet::of(&[CardType::Planeswalker])));
    cantrip_api::Game::add_counters(&mut cx, battle, CounterKind::Loyalty, 10);
    attacks_nothing(&cx, attacker, THEM);
    assert_eq!(damage_to(&mut cx, battle), 0);
}

/// The same for a planeswalker that becomes only a battle (it has a protector, its controller, as from an earlier time as
/// one, CR 310.9g): never both, so removed (CR 506.4).
#[test]
fn an_attacked_planeswalker_that_becomes_only_a_battle_is_dealt_no_combat_damage() {
    let (mut game, attacker, walker) = game(2, &WALKER, THEM, Some(THEM), THEM);
    let mut cx = cx(&mut game);
    effect(&mut cx, walker, EffectKind::SetTypes(TypeSet::of(&[CardType::Battle])));
    cantrip_api::Game::add_counters(&mut cx, walker, CounterKind::Defense, 10);
    attacks_nothing(&cx, attacker, THEM);
    assert_eq!(damage_to(&mut cx, walker), 0);
}

/// A planeswalker being attacked becomes a battle too - now a permanent being attacked that is both - and then stops being
/// a planeswalker: CR 506.4e keeps it a battle being attacked, and it is dealt the combat damage.
#[test]
fn an_attacked_planeswalker_that_becomes_both_and_then_only_a_battle_stays_attacked() {
    let (mut game, _, walker) = game(2, &WALKER, THEM, None, THEM);
    let mut cx = cx(&mut game);
    effect(&mut cx, walker, EffectKind::AddTypes(TypeSet::of(&[CardType::Battle])));
    assert_eq!(cx.s().attackers[0].target, Target::Object(walker), "both: still attacked");
    effect(&mut cx, walker, EffectKind::SetTypes(TypeSet::of(&[CardType::Battle])));
    assert_eq!(cx.s().attackers[0].target, Target::Object(walker), "CR 506.4e: still a battle being attacked");
    assert_eq!(damage_to(&mut cx, walker), 3);
}

/// Should some path ever fail to record a removal, the combat damage step still deals no damage to a permanent that is not
/// a planeswalker or battle on the battlefield: the combat state names one (written here as a missed removal would leave
/// it) that has become a creature alone with no settling since.
#[test]
fn a_permanent_named_by_the_combat_state_that_is_no_longer_attackable_is_dealt_no_combat_damage() {
    let (mut game, _, walker) = game(2, &WALKER, THEM, None, THEM);
    let timestamp = game.state.timestamp();
    game.state.effect_push(ContinuousEffect {
        source: walker,
        controller: THEM,
        timestamp,
        affected: walker,
        kind: EffectKind::SetTypes(TypeSet::of(&[CardType::Creature])),
        duration: Duration::EndOfTurn,
    });
    let mut cx = cx(&mut game);
    assert_eq!(damage_to(&mut cx, walker), 0);
}

/// `artifact` animated into a 3/3 artifact creature by an effect of `animator` that lasts while it is on the battlefield.
fn animate(game: &mut RulesGame, animator: ObjectId, artifact: ObjectId) {
    let mut cx = cx(game);
    for kind in [EffectKind::AddTypes(TypeSet::of(&[CardType::Creature])), EffectKind::SetPt(3, 3)] {
        cantrip_api::Game::add_effect(&mut cx, animator, artifact, kind, Duration::WhileSourceOnBattlefield);
    }
}

/// YOU's artifact, animated (Ensoul Artifact, March of the Machines), attacks; the enchantment animating it leaves before
/// combat damage, so it is no longer a creature: it is removed from combat (CR 506.4), no longer an attacking creature.
#[test]
fn an_attacking_creature_that_stops_being_a_creature_is_removed_from_combat() {
    let (mut game, _, _) = game(2, &WALKER, THEM, None, THEM);
    let artifact = put(&mut game, &ARTIFACT, YOU);
    let aura = put(&mut game, &ANIMATOR, YOU);
    animate(&mut game, aura, artifact);
    game.state.set_attackers(vec![attacking(artifact, Target::Player(THEM), THEM, TypeSet::NONE)]);
    let mut cx = cx(&mut game);
    assert!(cx.chars(artifact).is(CardType::Creature));
    crate::ops::move_to(&mut cx, aura, Zone::graveyard(YOU));
    assert!(!cx.chars(artifact).is(CardType::Creature));
    assert!(cx.s().attackers.iter().all(|a| a.id != artifact), "no longer an attacking creature");
}

/// THEM's animated artifact blocks YOU's lifelink creature; the enchantment animating it leaves, so it stops blocking (CR
/// 506.4), though an effect makes it a creature again before combat damage. The attacker stays blocked, by nothing: it
/// assigns no combat damage (CR 510.1c), and its controller gains no life.
#[test]
fn a_blocking_creature_that_stops_being_a_creature_and_becomes_one_again_blocks_no_more() {
    let (mut game, _, _) = game(2, &WALKER, THEM, None, THEM);
    let attacker = put(&mut game, &LIFELINKER, YOU);
    let artifact = put(&mut game, &ARTIFACT, THEM);
    let aura = put(&mut game, &ANIMATOR, THEM);
    animate(&mut game, aura, artifact);
    game.state.set_attackers(vec![Attacker { blocked: true, ..attacking(attacker, Target::Player(THEM), THEM, TypeSet::NONE) }]);
    game.state.set_blocks(vec![Block { blocker: artifact, attacker, first_strike: false }]);
    let mut cx = cx(&mut game);
    crate::ops::move_to(&mut cx, aura, Zone::graveyard(THEM));
    effect(&mut cx, artifact, EffectKind::AddTypes(TypeSet::of(&[CardType::Creature])));
    effect(&mut cx, artifact, EffectKind::SetPt(1, 1));
    assert!(cx.chars(artifact).is(CardType::Creature), "a creature again");
    assert!(cx.s().blocks.is_empty(), "it blocks no more");
    assert!(cx.s().attackers[0].blocked, "the attacker is still blocked");
    cx.st().update_turn(|t| t.set_step(Step::CombatDamage));
    combat_damage(&mut cx).unwrap();
    assert_eq!(cx.s().obj(artifact).damage, 0);
    assert_eq!(cx.s().player(YOU).life, 20, "no lifelink: no combat damage was dealt");
}

/// CR 506.4d: a planeswalker creature that blocks and is being attacked stops being a creature: it blocks no more, but is
/// still a planeswalker being attacked, and is dealt the combat damage of the creature attacking it; the creature it
/// blocked, blocked by nothing, deals none.
#[test]
fn a_blocking_planeswalker_creature_that_stops_being_a_creature_is_still_attacked() {
    let (mut game, _, _) = game(2, &WALKER, THEM, None, THEM);
    let first = put(&mut game, &ATTACKER, YOU);
    let blocked = put(&mut game, &LIFELINKER, YOU);
    let gideon = put(&mut game, &WALKER_CREATURE, THEM);
    game.state.set_attackers(vec![
        attacking(first, Target::Object(gideon), THEM, TypeSet::of(&[CardType::Planeswalker])),
        Attacker { blocked: true, ..attacking(blocked, Target::Player(THEM), THEM, TypeSet::NONE) },
    ]);
    game.state.set_blocks(vec![Block { blocker: gideon, attacker: blocked, first_strike: false }]);
    let mut cx = cx(&mut game);
    cantrip_api::Game::add_counters(&mut cx, gideon, CounterKind::Loyalty, 10);
    effect(&mut cx, gideon, EffectKind::SetTypes(TypeSet::of(&[CardType::Planeswalker])));
    assert!(cx.s().blocks.is_empty(), "it blocks no more");
    assert_eq!(cx.s().attackers.len(), 2);
    assert_eq!(cx.s().attackers[0].target, Target::Object(gideon), "still a planeswalker being attacked");
    assert_eq!(damage_to(&mut cx, gideon), 3);
    assert_eq!(cx.s().player(YOU).life, 20, "the blocked lifelink creature dealt nothing");
}

/// The defending player of `attacker`, which attacked `_walker`, as a card's ability asks it (Annihilator, Generous
/// Plunderer, Greatsword of Tyr: `Query::defending_player`).
fn defending_of(cx: &Cx, attacker: ObjectId, _walker: ObjectId) -> Option<PlayerId> {
    cantrip_api::Query::defending_player(cx, attacker)
}

/// YOU's creature attacks THEM's planeswalker; with its attack trigger on the stack, YOU gain control of the planeswalker
/// (Leyline of Anticipation and Mass Manipulation, both Pioneer). Its defending player is still THEM (CR 508.5; CR 506.2:
/// the nonactive player, never the attacking player), so Generous Plunderer's damage, Annihilator's sacrifice, and
/// Greatsword of Tyr's "creature defending player controls" are about THEM - and still once the creature has left combat.
#[test]
fn the_defending_player_is_the_one_attacked_though_the_planeswalker_changes_controller() {
    let (mut game, attacker, walker) = game(2, &WALKER, THEM, None, THEM);
    let mut cx = cx(&mut game);
    cx.emit(Event::Attacks { attacker, defender: Target::Object(walker), known: 0, defending: THEM });
    assert_eq!(defending_of(&cx, attacker, walker), Some(THEM));
    effect(&mut cx, walker, EffectKind::Control(YOU));
    assert_eq!(cx.s().obj(walker).controller, YOU);
    assert_eq!(defending_of(&cx, attacker, walker), Some(THEM), "the defending player it had as it attacked");
    crate::ops::move_to(&mut cx, attacker, Zone::graveyard(YOU));
    assert_eq!(defending_of(&cx, attacker, walker), Some(THEM), "and once it has left combat (CR 508.5)");
}
