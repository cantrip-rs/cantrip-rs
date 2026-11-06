//! A creature that can block more than one attacker (CR 509.1a): "can block any number of creatures" and "can block an
//! additional creature", from a static ability of its own (`CardDef::block_capacities`: Wall of Glare, Two-Headed Giant of
//! Foriys) or a resolved spell (`EffectKind::BlockCapacity`: Valor Made Real, Act of Heroism), which outlasts the creature
//! losing its abilities (CR 611.2c, 613.1f) where the static ability does not. "Blocks each attacking creature this turn if
//! able" (`EffectKind::MustBlockEach`, Blaze of Glory) is a requirement for each attacker (CR 509.1c). A creature blocking
//! several divides its combat damage among them as its controller chooses (CR 510.1d), triggers "whenever this creature
//! blocks" once (CR 509.3a) and "whenever it blocks a creature" for each (CR 509.3b), and one of those attackers leaving
//! combat leaves it blocking the rest. Over test cards; every declaration is judged by the core's validator.

use crate::{Game, Setup};
use cantrip_api::*;

const P0: PlayerId = PlayerId(0);
const P1: PlayerId = PlayerId(1);

fn creature_target(q: &dyn Query, _: &Asker, t: Target) -> bool {
    matches!(t, Target::Object(id) if q.chars(id).is(CardType::Creature))
}

/// "This creature can block any number of creatures."
fn any_number_itself(_: &dyn Query, source: ObjectId, blocker: ObjectId) -> Option<BlockCapacity> {
    (source == blocker).then_some(BlockCapacity::AnyNumber)
}

/// "This creature can block an additional creature each combat."
fn one_more_itself(_: &dyn Query, source: ObjectId, blocker: ObjectId) -> Option<BlockCapacity> {
    (source == blocker).then_some(BlockCapacity::Additional(1))
}

/// Each of `kinds` on the target until end of turn, a sentence each.
fn on_target(g: &mut dyn cantrip_api::Game, item: &StackItem, kinds: &[EffectKind]) {
    if let Target::Object(id) = item.target(0) {
        for (i, &kind) in kinds.iter().enumerate() {
            if i > 0 {
                g.end_sentence();
            }
            g.add_effect(item.object, id, kind, Duration::EndOfTurn);
        }
    }
}

/// "Target creature can block any number of creatures this turn. It blocks each attacking creature this turn if able."
fn blaze(g: &mut dyn cantrip_api::Game, item: &StackItem) -> Result<(), Suspend> {
    on_target(g, item, &[EffectKind::BlockCapacity(BlockCapacity::AnyNumber), EffectKind::MustBlockEach]);
    Ok(())
}

/// "Target creature can block any number of creatures this turn."
fn valor(g: &mut dyn cantrip_api::Game, item: &StackItem) -> Result<(), Suspend> {
    on_target(g, item, &[EffectKind::BlockCapacity(BlockCapacity::AnyNumber)]);
    Ok(())
}

/// "Target creature can block an additional creature this turn."
fn heroism(g: &mut dyn cantrip_api::Game, item: &StackItem) -> Result<(), Suspend> {
    on_target(g, item, &[EffectKind::BlockCapacity(BlockCapacity::Additional(1))]);
    Ok(())
}

/// "Destroy target creature."
fn kill(g: &mut dyn cantrip_api::Game, item: &StackItem) -> Result<(), Suspend> {
    if let Target::Object(id) = item.target(0) {
        g.destroy(id);
    }
    Ok(())
}

fn gain(g: &mut dyn cantrip_api::Game, item: &StackItem, amount: i32) -> Result<(), Suspend> {
    g.gain_life(item.controller, amount);
    Ok(())
}

fn gain_one(g: &mut dyn cantrip_api::Game, item: &StackItem) -> Result<(), Suspend> {
    gain(g, item, 1)
}

fn gain_ten(g: &mut dyn cantrip_api::Game, item: &StackItem) -> Result<(), Suspend> {
    gain(g, item, 10)
}

/// "Whenever this creature blocks" (CR 509.3a): its declared block alone.
fn this_blocks(_: &dyn Query, source: ObjectId, event: &Event) -> Option<Fired> {
    match *event {
        Event::Blocks { blocker, declared: true, .. } if blocker == source => Fired::PLAIN,
        _ => None,
    }
}

/// "Whenever this creature blocks a creature" (CR 509.3b): each attacker it blocks.
fn this_blocks_a_creature(_: &dyn Query, source: ObjectId, event: &Event) -> Option<Fired> {
    match *event {
        Event::Blocks { blocker, .. } if blocker == source => Fired::PLAIN,
        _ => None,
    }
}

static CREATURE: [TargetSpec; 1] = [target(creature_target)];
const fn instant(name: &'static str, effect: Effect) -> CardDef {
    CardDef { name, types: TypeSet::of(&[CardType::Instant]), spell: spell(&CREATURE, effect), ..CardDef::BLANK }
}
const fn creature(name: &'static str, power: i32, toughness: i32, keywords: Keywords) -> CardDef {
    CardDef { name, types: TypeSet::of(&[CardType::Creature]), power: Some(power), toughness: Some(toughness), keywords, ..CardDef::BLANK }
}
static FILLER: CardDef = CardDef { name: "Multi Block Test Filler", has_cost: false, types: TypeSet::of(&[CardType::Land]), ..CardDef::BLANK };
static BLAZE: CardDef = instant("Multi Block Test Blaze", blaze);
static VALOR: CardDef = instant("Multi Block Test Valor", valor);
static HEROISM: CardDef = instant("Multi Block Test Heroism", heroism);
static KILL: CardDef = instant("Multi Block Test Kill", kill);
static BEAR: CardDef = creature("Multi Block Test Bear", 2, 2, Keywords::NONE);
static FLYER: CardDef = creature("Multi Block Test Flyer", 2, 2, Keywords::of(&[Keyword::Flying]));
static WALL: CardDef = CardDef { block_capacities: &[any_number_itself], ..creature("Multi Block Test Wall", 4, 8, Keywords::NONE) };
static TWO_HEADED: CardDef = CardDef { block_capacities: &[one_more_itself], ..creature("Multi Block Test Two-Headed", 2, 4, Keywords::NONE) };
static WATCHER: CardDef = CardDef {
    abilities: &[
        triggered(when(EventMask::BLOCKS, this_blocks), gain_one, "Whenever this creature blocks, you gain 1 life."),
        triggered(when(EventMask::BLOCKS, this_blocks_a_creature), gain_ten, "Whenever this creature blocks a creature, you gain 10 life."),
    ],
    ..WALL
};

fn generated_extra_block(_: &dyn Query, source: ObjectId, blocker: ObjectId) -> u32 {
    u32::from(source == blocker)
}
static COMPOSED: CardDef = CardDef {
    name: "Multi Block Test Composed",
    abilities: &[block_allowance(1, "This creature can block an additional creature.")],
    additional_blocks: &[generated_extra_block],
    ..TWO_HEADED
};

fn table() -> &'static CardTable {
    Box::leak(Box::new(CardTable::new([&FILLER, &BLAZE, &VALOR, &HEROISM, &KILL, &BEAR, &FLYER, &WALL, &TWO_HEADED, &WATCHER, &COMPOSED])))
}

/// A game of two on turn 1, each deck starting with `spells` (P0's, then P1's), and `permanents` on the battlefield.
fn game(spells: [&[&'static CardDef]; 2], permanents: &[(&'static CardDef, PlayerId)]) -> (Game, Vec<ObjectId>) {
    let table = table();
    let filler = table.id_of(&FILLER);
    let deck = |spells: &[&'static CardDef]| {
        let mut d: Vec<CardId> = spells.iter().map(|&s| table.id_of(s)).collect();
        d.extend(std::iter::repeat_n(filler, 30));
        d
    };
    let mut g = Game::new(table, &Setup { decks: vec![deck(spells[0]), deck(spells[1])], ..Default::default() });
    g.verify = true;
    let ids = permanents
        .iter()
        .map(|&(def, owner)| {
            let id = g.state.push_obj(Obj::new(table.id_of(def), owner, Zone::BATTLEFIELD));
            g.state.zone_push(Zone::BATTLEFIELD, id);
            id
        })
        .collect();
    (g, ids)
}

/// Runs the game, passing and declaring nothing, until `stop` says the question asked is the one wanted.
fn until(g: &mut Game, stop: impl Fn(&Game, &Request) -> bool) -> Request {
    for _ in 0..200 {
        g.run();
        let request = g.pending().cloned().expect("a question");
        if stop(g, &request) {
            return request;
        }
        let answer = match &request.kind {
            RequestKind::Priority { .. } => Answer::Pass,
            RequestKind::DeclareAttackers { .. } => Answer::Attack(Vec::new()),
            RequestKind::DiscardToHandSize { count, hand } => Answer::Cards(hand.iter().copied().take(*count as usize).collect()),
            RequestKind::OrderTriggers { sources, .. } => Answer::Order((0..sources.len() as u32).collect()),
            other => panic!("unexpected question {other:?}"),
        };
        g.answer(request.id, request.player, answer).unwrap();
    }
    panic!("the question never came");
}

/// `player` casts `spell` from their hand at `target`, at their first priority `when` allows, and it resolves.
fn cast(g: &mut Game, player: PlayerId, spell: &'static CardDef, target: ObjectId, when: impl Fn(&Game) -> bool) {
    let card = g.cards.id_of(spell);
    let ask = until(g, |g, r| r.player == player && when(g) && matches!(r.kind, RequestKind::Priority { .. }));
    let RequestKind::Priority { options } = &ask.kind else { unreachable!() };
    let option = options
        .iter()
        .find(|o| o.kind == ActionKind::CastSpell && o.source.is_some_and(|s| g.state.obj(s).card == card))
        .expect("the spell is offered");
    let mut action = Action::of(option);
    action.targets = vec![Target::Object(target)];
    g.answer(ask.id, player, Answer::Act(action)).unwrap();
    until(g, |g, r| g.state.stack.is_empty() && matches!(r.kind, RequestKind::Priority { .. }));
}

fn main_phase(g: &Game) -> bool {
    g.state.turn.step() == Step::PrecombatMain
}

/// P0 attacks P1 with `attackers`: the declaration of blockers P1 is asked for.
fn declare(g: &mut Game, attackers: &[ObjectId]) -> Request {
    let attack = until(g, |_, r| matches!(r.kind, RequestKind::DeclareAttackers { .. }));
    g.answer(attack.id, P0, Answer::Attack(attackers.iter().map(|&a| (a, Target::Player(P1))).collect())).unwrap();
    until(g, |_, r| matches!(r.kind, RequestKind::DeclareBlockers { .. }))
}

fn rules(request: &Request) -> &BlockRequirements {
    let RequestKind::DeclareBlockers { requirements, .. } = &request.kind else { panic!("not a declaration of blockers") };
    requirements
}

fn legal(g: &Game, request: &Request, blocks: &[(ObjectId, ObjectId)]) -> bool {
    crate::check_answer(g, request, &Answer::Block(blocks.to_vec())).is_ok()
}

fn block(g: &mut Game, request: &Request, blocks: &[(ObjectId, ObjectId)]) {
    g.answer(request.id, P1, Answer::Block(blocks.to_vec())).unwrap();
}

/// The next question past the declaration of blockers that is no priority: the division of combat damage, or none once
/// combat damage has been dealt.
fn past_blocks(g: &mut Game) -> Option<Request> {
    let next = until(g, |g, r| matches!(r.kind, RequestKind::AssignCombatDamage { .. }) || g.state.turn.step() > Step::CombatDamage);
    matches!(next.kind, RequestKind::AssignCombatDamage { .. }).then_some(next)
}

/// A creature that can block any number of creatures (its own static ability) blocks three attackers, each a block of its
/// own; a creature that can block one may not block two.
#[test]
fn a_creature_that_can_block_any_number_blocks_every_attacker() {
    let (mut g, ids) = game([&[], &[]], &[(&BEAR, P0), (&BEAR, P0), (&BEAR, P0), (&WALL, P1), (&BEAR, P1)]);
    let (a1, a2, a3, wall, bear) = (ids[0], ids[1], ids[2], ids[3], ids[4]);
    let ask = declare(&mut g, &[a1, a2, a3]);
    assert_eq!(rules(&ask).capacity_of(wall), u32::MAX);
    assert_eq!(rules(&ask).capacity_of(bear), 1);
    assert!(!legal(&g, &ask, &[(bear, a1), (bear, a2)]));
    assert!(!legal(&g, &ask, &[(wall, a1), (wall, a1)]), "an attacker is blocked once by a creature");
    assert!(legal(&g, &ask, &[(wall, a1), (wall, a2), (wall, a3), (bear, a3)]));
    block(&mut g, &ask, &[(wall, a1), (wall, a2), (wall, a3)]);
    assert_eq!(g.state.blocks.iter().filter(|b| b.blocker == wall).count(), 3);
    assert!(g.state.attackers.iter().all(|a| a.blocked));
    // Its 4 damage is divided among the three as P1 chooses (CR 510.1d): 2, 2 and 0.
    let divide = past_blocks(&mut g).expect("the wall divides its damage");
    assert_eq!(divide.player, P1);
    let RequestKind::AssignCombatDamage { divisions } = &divide.kind else { unreachable!() };
    assert_eq!(divisions.len(), 1);
    assert_eq!((divisions[0].source, divisions[0].damage, divisions[0].creatures.clone()), (wall, 4, vec![a1, a2, a3]));
    g.answer(divide.id, P1, Answer::Shares(vec![(wall, Some(a1), 2), (wall, Some(a3), 2)])).unwrap();
    until(&mut g, |g, _| g.state.turn.step() > Step::CombatDamage);
    assert_eq!(g.state.players[1].life, 20, "every attacker was blocked");
    assert_eq!(g.state.obj(wall).damage, 6);
    let here = |id: ObjectId| g.state.battlefield().contains(&id);
    assert!(!here(a1) && !here(a3) && here(a2));
}

/// "Can block an additional creature" makes two: a third block is refused.
#[test]
fn an_additional_creature_makes_two() {
    let (mut g, ids) = game([&[], &[]], &[(&BEAR, P0), (&BEAR, P0), (&BEAR, P0), (&TWO_HEADED, P1)]);
    let (a1, a2, a3, giant) = (ids[0], ids[1], ids[2], ids[3]);
    let ask = declare(&mut g, &[a1, a2, a3]);
    assert_eq!(rules(&ask).capacity_of(giant), 2);
    assert!(legal(&g, &ask, &[(giant, a1), (giant, a3)]));
    assert!(!legal(&g, &ask, &[(giant, a1), (giant, a2), (giant, a3)]));
}

/// Additional creatures add up (CR 509.1a): a creature that can block an additional one, given another this turn, blocks
/// three; and "any number" from a resolved spell lets a creature with no such ability block them all.
#[test]
fn capacities_add_up_and_a_resolved_one_widens_any_creature() {
    let (mut g, ids) = game([&[&HEROISM, &VALOR], &[]], &[(&BEAR, P0), (&BEAR, P0), (&BEAR, P0), (&BEAR, P0), (&TWO_HEADED, P1), (&BEAR, P1)]);
    let (a1, a2, a3, a4, giant, bear) = (ids[0], ids[1], ids[2], ids[3], ids[4], ids[5]);
    cast(&mut g, P0, &HEROISM, giant, main_phase);
    cast(&mut g, P0, &VALOR, bear, main_phase);
    let ask = declare(&mut g, &[a1, a2, a3, a4]);
    assert_eq!(rules(&ask).capacity_of(giant), 3);
    assert_eq!(rules(&ask).capacity_of(bear), u32::MAX);
    assert!(!legal(&g, &ask, &[(giant, a1), (giant, a2), (giant, a3), (giant, a4)]));
    assert!(legal(&g, &ask, &[(giant, a1), (giant, a2), (giant, a3), (bear, a1), (bear, a2), (bear, a3), (bear, a4)]));
}

/// A resolved "can block any number of creatures this turn" is an effect on the creature, no ability of it (CR 113.1a,
/// 611.2c): the creature losing all its abilities (CR 613.1f) still can. A static ability of its own is an ability, and
/// the same loss takes it.
#[test]
fn a_resolved_capacity_outlasts_losing_all_abilities_and_a_static_one_does_not() {
    let (mut g, ids) = game([&[&VALOR], &[]], &[(&BEAR, P0), (&BEAR, P0), (&BEAR, P1), (&WALL, P1)]);
    let (a1, a2, bear, wall) = (ids[0], ids[1], ids[2], ids[3]);
    // The loss comes first and the spell after it: an effect is no ability, whatever its timestamp.
    for id in [bear, wall] {
        let timestamp = g.state.timestamp();
        g.state.effect_push(ContinuousEffect {
            source: id,
            controller: P0,
            timestamp,
            affected: id,
            kind: EffectKind::LoseAllAbilities,
            duration: Duration::EndOfTurn,
            words: Words::NONE,
        });
    }
    cast(&mut g, P0, &VALOR, bear, main_phase);
    let ask = declare(&mut g, &[a1, a2]);
    assert_eq!(rules(&ask).capacity_of(bear), u32::MAX);
    assert_eq!(rules(&ask).capacity_of(wall), 1);
    assert!(legal(&g, &ask, &[(bear, a1), (bear, a2)]));
    assert!(!legal(&g, &ask, &[(wall, a1), (wall, a2)]));
}

/// "It blocks each attacking creature this turn if able" (CR 509.1c): a requirement for each attacker it may block, and
/// the declaration must obey as many as any can - all of them. The flyer it can't block makes no requirement.
#[test]
fn a_creature_that_blocks_each_attacker_must_block_every_one_it_can() {
    let (mut g, ids) = game([&[&BLAZE], &[]], &[(&BEAR, P0), (&BEAR, P0), (&FLYER, P0), (&BEAR, P1), (&BEAR, P1)]);
    let (a1, a2, flyer, x, y) = (ids[0], ids[1], ids[2], ids[3], ids[4]);
    cast(&mut g, P0, &BLAZE, x, main_phase);
    let ask = declare(&mut g, &[a1, a2, flyer]);
    let required = &rules(&ask).required;
    assert_eq!(required.len(), 2);
    assert!(required.contains(&RequiredBlock { blocker: Some(x), attacker: Some(a1) }));
    assert!(required.contains(&RequiredBlock { blocker: Some(x), attacker: Some(a2) }));
    assert_eq!(rules(&ask).most, 2);
    let suggested = rules(&ask).suggested.clone();
    assert!(suggested.contains(&(x, a1)) && suggested.contains(&(x, a2)) && legal(&g, &ask, &suggested));
    assert!(!legal(&g, &ask, &[]));
    assert!(!legal(&g, &ask, &[(x, a1), (y, a2)]));
    assert!(legal(&g, &ask, &[(x, a1), (x, a2)]) && legal(&g, &ask, &[(x, a1), (x, a2), (y, a2)]));
}

/// A creature blocking three attackers triggers "whenever this creature blocks" once (CR 509.3a) and "whenever this creature
/// blocks a creature" for each (CR 509.3b).
#[test]
fn blocking_several_triggers_blocks_once_and_blocks_a_creature_for_each() {
    let (mut g, ids) = game([&[], &[]], &[(&BEAR, P0), (&BEAR, P0), (&BEAR, P0), (&WATCHER, P1)]);
    let (a1, a2, a3, watcher) = (ids[0], ids[1], ids[2], ids[3]);
    let ask = declare(&mut g, &[a1, a2, a3]);
    block(&mut g, &ask, &[(watcher, a1), (watcher, a2), (watcher, a3)]);
    until(&mut g, |g, r| g.state.stack.is_empty() && g.state.turn.step() == Step::DeclareBlockers && r.player == P1 && matches!(r.kind, RequestKind::Priority { .. }));
    assert_eq!(g.state.players[1].life, 20 + 1 + 30);
}

/// One of the attackers a creature blocks leaving combat leaves it blocking the other (CR 506.4, 510.1d): all of its damage
/// goes to that one, with nothing to divide.
#[test]
fn an_attacker_leaving_combat_leaves_the_blocker_blocking_the_rest() {
    let (mut g, ids) = game([&[], &[&KILL]], &[(&BEAR, P0), (&BEAR, P0), (&WALL, P1)]);
    let (a1, a2, wall) = (ids[0], ids[1], ids[2]);
    let ask = declare(&mut g, &[a1, a2]);
    block(&mut g, &ask, &[(wall, a1), (wall, a2)]);
    cast(&mut g, P1, &KILL, a2, |g| g.state.turn.step() == Step::DeclareBlockers);
    assert!(past_blocks(&mut g).is_none(), "one creature left to deal damage to");
    assert!(!g.state.battlefield().contains(&a1));
    assert_eq!(g.state.obj(wall).damage, 2);
}

/// Generator callbacks, gained-ability recipes, and upstream static/resolved capacities use the same solver.
#[test]
fn generated_and_upstream_block_allowances_compose_once() {
    for resolved in [false, true] {
        let (mut g, ids) = game([&[&HEROISM], &[]], &[
            (&BEAR, P0), (&BEAR, P0), (&BEAR, P0), (&BEAR, P0), (&BEAR, P0), (&COMPOSED, P1),
        ]);
        let blocker = ids[5];
        if resolved { cast(&mut g, P0, &HEROISM, blocker, main_phase); }
        let ask = declare(&mut g, &ids[..5]);
        assert_eq!(rules(&ask).capacity_of(blocker), if resolved { 5 } else { 4 });
        let blocks: Vec<_> = ids[..5].iter().map(|&attacker| (blocker, attacker)).collect();
        assert!(legal(&g, &ask, &blocks[..4]));
        assert_eq!(legal(&g, &ask, &blocks), resolved);
    }
}
