//! An activated ability that divides a total among its targets (CR 601.2d by way of 602.2b), over test cards: the targets and
//! their shares are announced as it is activated, after X and before its total cost (CR 601.2c-f), each target getting at
//! least one; a total read from the game is locked in there; a target that has become illegal loses its share and the rest
//! is dealt as announced (CR 608.2b); and a copy keeps the division, its new targets taking the old ones' shares in turn
//! (CR 707.10, 115.7f). A loyalty ability pays its loyalty once the division is announced (CR 606.4, 601.2h). The printed
//! cases are Vivien, Arkbow Ranger, Arc Mage, Living Inferno, Lukka, Bound to Ruin and Captain America's Throw.
use crate::actions::priority_options;
use crate::{Cx, Game, Setup};
use cantrip_api::*;
use cantrip_state::{Frame, FrameKind};

const YOU: PlayerId = PlayerId(0);
const THEM: PlayerId = PlayerId(1);

static FILLER: CardDef = CardDef { name: "Divide Test Filler", has_cost: false, types: TypeSet::of(&[CardType::Land]), ..CardDef::BLANK };

static BEAR: CardDef = CardDef {
    name: "Divide Test Bear",
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(2),
    toughness: Some(2),
    ..CardDef::BLANK
};

fn creature(q: &dyn Query, _: &Asker, t: Target) -> bool {
    matches!(t, Target::Object(id) if q.obj(id).zone.is(ZoneKind::Battlefield) && q.chars(id).is(CardType::Creature))
}

fn distribute_plus_ones(g: &mut dyn cantrip_api::Game, item: &StackItem) -> Result<(), Suspend> {
    cantrip_mechanics::counters_divided(g, item, CounterKind::PlusOne);
    Ok(())
}

fn damage_divided(g: &mut dyn cantrip_api::Game, item: &StackItem) -> Result<(), Suspend> {
    cantrip_mechanics::damage_divided(g, item);
    Ok(())
}

/// A planeswalker: "+1: Distribute two +1/+1 counters among up to two target creatures." (Vivien, Arkbow Ranger's.)
static RANGER: CardDef = CardDef {
    name: "Divide Test Ranger",
    types: TypeSet::of(&[CardType::Planeswalker]),
    loyalty: Some(3),
    abilities: &[ability(loyalty(1), distribute_plus_ones, "+1: Distribute two +1/+1 counters among up to two target creatures.")
        .targets(&[up_to(2, creature)])
        .divide(Divide { total: 2, reads: None })],
    ..CardDef::BLANK
};

/// An artifact: "{1}: This artifact deals 3 damage divided as you choose among one or two targets." (Arc Mage's, without its
/// other costs.)
static ARC: CardDef = CardDef {
    name: "Divide Test Arc",
    types: TypeSet::of(&[CardType::Artifact]),
    abilities: &[ability(cost("{1}"), damage_divided, "{1}: This artifact deals 3 damage divided as you choose among one or two targets.")
        .targets(&[cantrip_mechanics::targets::ANY.between(1, 2)])
        .divide(Divide { total: 3, reads: None })],
    ..CardDef::BLANK
};

/// Its source's power as the division is announced, none if it is negative.
fn its_power(q: &dyn Query, asker: &Asker, _: u32) -> u32 {
    u32::try_from(q.chars(asker.source).power.unwrap_or(0)).unwrap_or(0)
}

/// A 3/6: "{0}: This creature deals damage equal to its power divided as you choose among any number of target creatures."
/// (Living Inferno's, without its tap.)
static INFERNO: CardDef = CardDef {
    name: "Divide Test Inferno",
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(3),
    toughness: Some(6),
    abilities: &[ability(Cost::NONE, damage_divided, "{0}: This creature deals damage equal to its power divided as you choose among any number of target creatures.")
        .targets(&[up_to(TargetSpec::ANY_NUMBER, creature)])
        .divide(Divide { total: TargetSpec::COUNTED, reads: Some(its_power) })],
    ..CardDef::BLANK
};

/// A 0/3: "{0}: This creature deals damage equal to its power divided as you choose among one or two target creatures."
static EMBER: CardDef = CardDef {
    name: "Divide Test Ember",
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(0),
    toughness: Some(3),
    abilities: &[ability(Cost::NONE, damage_divided, "{0}: This creature deals damage equal to its power divided as you choose among one or two target creatures.")
        .targets(&[target(creature).between(1, 2)])
        .divide(Divide { total: TargetSpec::COUNTED, reads: Some(its_power) })],
    ..CardDef::BLANK
};

fn ability_on_stack(_: &dyn Query, _: &Asker, t: Target) -> bool {
    matches!(t, Target::Ability(_))
}

fn copy_with_new_targets(g: &mut dyn cantrip_api::Game, item: &StackItem) -> Result<(), Suspend> {
    let ability = g.state().target(item, 0);
    g.copy_ability(ability, item.controller, true)?;
    Ok(())
}

/// A {0} instant: "Copy target activated ability. You may choose new targets for the copy." (Rings of Brighthearth's copy, as
/// a spell.)
static ECHO: CardDef = CardDef {
    name: "Divide Test Echo",
    types: TypeSet::of(&[CardType::Instant]),
    spell: spell(&[target(ability_on_stack)], copy_with_new_targets),
    ..CardDef::BLANK
};

/// Your precombat main phase with priority, nothing anywhere.
fn game() -> Game {
    let table: &'static CardTable = Box::leak(Box::new(CardTable::new([&FILLER, &BEAR, &RANGER, &ARC, &INFERNO, &EMBER, &ECHO])));
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
    g.state.zone_push(zone, id);
    id
}

fn cx(game: &mut Game) -> Cx<'_> {
    Cx { g: game, frame: usize::MAX, cursor: 0, mana_source: None, entering: None, lineage: Vec::new(), damage: None, asked_at: None, doer: None, producing: None }
}

fn next(g: &mut Game) -> Request {
    g.run();
    g.pending().cloned().expect("a question")
}

fn answer(g: &mut Game, r: &Request, a: Answer) {
    g.answer(r.id, r.player, a).expect("a legal answer");
}

fn offered(g: &mut Game, source: ObjectId) -> Option<ActionOption> {
    priority_options(&mut cx(g), YOU).into_iter().find(|o| o.source == Some(source))
}

/// Takes the action offered for `source` and gives back the question after it: the division, for these abilities.
fn activate(g: &mut Game, source: ObjectId) -> Request {
    let option = offered(g, source).expect("the ability is offered");
    assert!(option.target_candidates.is_empty(), "a divided slot is never announced with the action (CR 601.2d)");
    let r = next(g);
    answer(g, &r, Answer::Act(Action::of(&option)));
    next(g)
}

/// Pays what is asked from the pool and passes until the stack is empty.
fn pay_and_resolve(g: &mut Game, mut r: Request) {
    if let RequestKind::PayCosts { options, .. } = &r.kind {
        let confirm = options.iter().find(|o| o.kind == ActionKind::ConfirmCasting).expect("the pool pays it");
        answer(g, &r, Answer::Act(Action::of(confirm)));
        r = next(g);
    }
    while !g.state.stack.is_empty() {
        assert!(matches!(r.kind, RequestKind::Priority { .. }), "expected priority, got {:?}", r.kind);
        answer(g, &r, Answer::Pass);
        r = next(g);
    }
}

fn plus_ones(g: &Game, id: ObjectId) -> i32 {
    g.state.obj(id).counters(CounterKind::PlusOne)
}

#[test]
fn a_loyalty_ability_divides_its_counters_before_its_loyalty_is_paid() {
    let mut g = game();
    let theirs = put(&mut g, &BEAR, THEM, Zone::BATTLEFIELD);
    let yours = put(&mut g, &BEAR, YOU, Zone::BATTLEFIELD);
    let ranger = put(&mut g, &RANGER, YOU, Zone::BATTLEFIELD);
    crate::counters::put(&mut cx(&mut g), ranger, CounterKind::Loyalty, 3, YOU, crate::counters::How::Effect);
    let divide = activate(&mut g, ranger);
    let RequestKind::ChooseDividedTargets { source, candidates, total, max_targets, optional, .. } = &divide.kind else {
        panic!("the division is asked as it is activated: {:?}", divide.kind)
    };
    assert_eq!((*source, *total, *max_targets, *optional), (ranger, 2, 2, true));
    assert!(candidates.contains(&Target::Object(theirs)) && candidates.contains(&Target::Object(yours)));
    assert_eq!(g.state.obj(ranger).counters(CounterKind::Loyalty), 3, "the loyalty cost is paid after the division (CR 601.2d, 601.2h)");
    // Two counters, never one each and a creature with none (CR 601.2d).
    assert!(g.answer(divide.id, divide.player, Answer::Divided(vec![(Target::Object(yours), 2), (Target::Object(theirs), 0)])).is_err());
    let before = g.at;
    answer(&mut g, &divide, Answer::Divided(vec![(Target::Object(yours), 1), (Target::Object(theirs), 1)]));
    let after = next(&mut g);
    assert_eq!(g.state.obj(ranger).counters(CounterKind::Loyalty), 4, "+1 paid as the cost (CR 606.4)");
    let item = *g.state.stack.last().unwrap();
    assert_eq!(g.state.targets(&item), &[Target::Object(yours), Target::Object(theirs)]);
    assert_eq!(g.state.shares(&item), &[1, 1]);
    // Undone and done again, as the search goes over it.
    let at = g.at;
    g.goto(before);
    assert!(matches!(g.pending().unwrap().kind, RequestKind::ChooseDividedTargets { .. }));
    assert_eq!(g.state.obj(ranger).counters(CounterKind::Loyalty), 3);
    g.goto(at);
    pay_and_resolve(&mut g, after);
    assert_eq!((plus_ones(&g, yours), plus_ones(&g, theirs)), (1, 1));
}

#[test]
fn up_to_two_targets_may_be_none_and_the_counters_go_nowhere() {
    let mut g = game();
    let yours = put(&mut g, &BEAR, YOU, Zone::BATTLEFIELD);
    let ranger = put(&mut g, &RANGER, YOU, Zone::BATTLEFIELD);
    crate::counters::put(&mut cx(&mut g), ranger, CounterKind::Loyalty, 3, YOU, crate::counters::How::Effect);
    let divide = activate(&mut g, ranger);
    answer(&mut g, &divide, Answer::Divided(Vec::new()));
    let r = next(&mut g);
    assert_eq!(g.state.obj(ranger).counters(CounterKind::Loyalty), 4);
    assert_eq!(g.state.stack.last().map(|i| i.target_count), Some(0));
    pay_and_resolve(&mut g, r);
    assert_eq!(plus_ones(&g, yours), 0);
    // A creature that is no planeswalker's ability's to give is not even needed: with no creature at all it is still offered.
    let mut g = game();
    let ranger = put(&mut g, &RANGER, YOU, Zone::BATTLEFIELD);
    crate::counters::put(&mut cx(&mut g), ranger, CounterKind::Loyalty, 3, YOU, crate::counters::How::Effect);
    assert!(offered(&mut g, ranger).is_some(), "\"up to two\" with no creature: activated with no target (CR 601.2c)");
}

#[test]
fn a_fixed_total_above_its_targets_goes_whole_to_one_and_is_asked_before_the_mana() {
    let mut g = game();
    let theirs = put(&mut g, &BEAR, THEM, Zone::BATTLEFIELD);
    let arc = put(&mut g, &ARC, YOU, Zone::BATTLEFIELD);
    g.state.add_mana(YOU, Mana::C, 1, ObjectId(0), None, ManaLasts::Step, false);
    let divide = activate(&mut g, arc);
    let RequestKind::ChooseDividedTargets { total, max_targets, optional, candidates, .. } = &divide.kind else {
        panic!("the division is asked first: {:?}", divide.kind)
    };
    assert_eq!((*total, *max_targets, *optional), (3, 2, false));
    assert!(candidates.contains(&Target::Player(THEM)), "any target: a player too");
    assert!(g.answer(divide.id, divide.player, Answer::Divided(Vec::new())).is_err(), "one or two targets: not none");
    assert_eq!(g.state.player(YOU).pool.total(), 1, "the mana is paid once the division is announced (CR 601.2f-h)");
    answer(&mut g, &divide, Answer::Divided(vec![(Target::Object(theirs), 3)]));
    let r = next(&mut g);
    assert_eq!(g.state.player(YOU).pool.total(), 0);
    pay_and_resolve(&mut g, r);
    assert!(!g.state.battlefield().contains(&theirs), "3 damage to the 2/2");
}

#[test]
fn a_target_gone_illegal_loses_its_share_and_the_rest_is_dealt() {
    let mut g = game();
    let theirs = put(&mut g, &BEAR, THEM, Zone::BATTLEFIELD);
    let arc = put(&mut g, &ARC, YOU, Zone::BATTLEFIELD);
    g.state.add_mana(YOU, Mana::C, 1, ObjectId(0), None, ManaLasts::Step, false);
    let life = g.state.player(THEM).life;
    let divide = activate(&mut g, arc);
    answer(&mut g, &divide, Answer::Divided(vec![(Target::Object(theirs), 2), (Target::Player(THEM), 1)]));
    let pay = next(&mut g);
    if let RequestKind::PayCosts { options, .. } = &pay.kind {
        let confirm = options.iter().find(|o| o.kind == ActionKind::ConfirmCasting).expect("the pool pays it");
        answer(&mut g, &pay, Answer::Act(Action::of(confirm)));
    }
    let r = next(&mut g);
    // The Bear leaves in response: its 2 is lost, not dealt to the player (CR 608.2b).
    crate::ops::move_to(&mut cx(&mut g), theirs, Zone::graveyard(THEM));
    pay_and_resolve(&mut g, r);
    assert_eq!(g.state.player(THEM).life, life - 1);
}

#[test]
fn a_total_read_from_the_game_is_locked_in_as_it_is_activated() {
    let mut g = game();
    let theirs = put(&mut g, &BEAR, THEM, Zone::BATTLEFIELD);
    let other = put(&mut g, &BEAR, THEM, Zone::BATTLEFIELD);
    let inferno = put(&mut g, &INFERNO, YOU, Zone::BATTLEFIELD);
    let divide = activate(&mut g, inferno);
    let RequestKind::ChooseDividedTargets { total, max_targets, optional, .. } = &divide.kind else { panic!("{:?}", divide.kind) };
    assert_eq!((*total, *max_targets, *optional), (3, 0, true), "its power as it is activated; any number of targets");
    assert!(g.answer(divide.id, divide.player, Answer::Divided(vec![(Target::Object(theirs), 3), (Target::Object(other), 1)])).is_err());
    answer(&mut g, &divide, Answer::Divided(vec![(Target::Object(theirs), 2), (Target::Object(other), 1)]));
    let r = next(&mut g);
    // Its power drops before it resolves: the damage stays as divided (Living Inferno's ruling).
    crate::counters::put(&mut cx(&mut g), inferno, CounterKind::MinusOne, 3, YOU, crate::counters::How::Effect);
    pay_and_resolve(&mut g, r);
    assert!(!g.state.battlefield().contains(&theirs));
    assert_eq!(g.state.obj(other).damage, 1);
}

#[test]
fn a_total_of_nothing_is_no_announcement_of_a_slot_that_requires_a_target() {
    let mut g = game();
    put(&mut g, &BEAR, THEM, Zone::BATTLEFIELD);
    let ember = put(&mut g, &EMBER, YOU, Zone::BATTLEFIELD);
    assert!(offered(&mut g, ember).is_none(), "power 0 among one or two targets: each must receive at least one (CR 601.2d)");
    // Any number of targets may be none: power 0 is activated, and divides nothing.
    let mut g = game();
    let theirs = put(&mut g, &BEAR, THEM, Zone::BATTLEFIELD);
    let inferno = put(&mut g, &INFERNO, YOU, Zone::BATTLEFIELD);
    crate::counters::put(&mut cx(&mut g), inferno, CounterKind::MinusOne, 3, YOU, crate::counters::How::Effect);
    let option = offered(&mut g, inferno).expect("offered with a total of nothing");
    let r = next(&mut g);
    answer(&mut g, &r, Answer::Act(Action::of(&option)));
    let r = next(&mut g);
    assert!(matches!(r.kind, RequestKind::Priority { .. }), "nothing to divide is not asked: {:?}", r.kind);
    assert_eq!(g.state.stack.last().map(|i| i.target_count), Some(0));
    pay_and_resolve(&mut g, r);
    assert_eq!(g.state.obj(theirs).damage, 0);
}

#[test]
fn a_copy_keeps_the_division_and_its_new_targets_take_the_shares_in_turn() {
    let mut g = game();
    let theirs = put(&mut g, &BEAR, THEM, Zone::BATTLEFIELD);
    let yours = put(&mut g, &BEAR, YOU, Zone::BATTLEFIELD);
    let arc = put(&mut g, &ARC, YOU, Zone::BATTLEFIELD);
    let echo = put(&mut g, &ECHO, YOU, Zone::hand(YOU));
    g.state.add_mana(YOU, Mana::C, 1, ObjectId(0), None, ManaLasts::Step, false);
    let life = g.state.player(THEM).life;
    let divide = activate(&mut g, arc);
    answer(&mut g, &divide, Answer::Divided(vec![(Target::Object(theirs), 2), (Target::Player(THEM), 1)]));
    let pay = next(&mut g);
    if let RequestKind::PayCosts { options, .. } = &pay.kind {
        let confirm = options.iter().find(|o| o.kind == ActionKind::ConfirmCasting).expect("the pool pays it");
        answer(&mut g, &pay, Answer::Act(Action::of(confirm)));
    }
    let serial = g.state.stack.last().unwrap().serial;
    // Copied with the same targets: the division with them (CR 707.10).
    crate::ops::copy_ability(&mut cx(&mut g), Target::Ability(serial), YOU, false).expect("copied");
    let (original, copy) = (g.state.stack[0], *g.state.stack.last().unwrap());
    assert_eq!(g.state.targets(&copy), g.state.targets(&original));
    assert_eq!(g.state.shares(&copy), &[2, 1]);
    g.state.stack_remove(g.state.stack.len() - 1);
    // Copied by a spell that lets its controller choose new targets: the shares stay where they were announced (CR 115.7f).
    let r = next(&mut g);
    let option = priority_options(&mut cx(&mut g), YOU).into_iter().find(|o| o.source == Some(echo)).expect("the copy spell is offered");
    let option = ActionOption { target_candidates: vec![Target::Ability(serial)], ..option };
    answer(&mut g, &r, Answer::Act(Action { targets: vec![Target::Ability(serial)], ..Action::of(&option) }));
    let mut r = next(&mut g);
    loop {
        match &r.kind {
            RequestKind::ChooseTarget { replacing: Some((index, original)), .. } => {
                let choices = match (*index, *original) {
                    (0, Target::Object(id)) if id == theirs => vec![Target::Object(yours)],
                    (1, Target::Player(player)) if player == THEM => Vec::new(),
                    other => panic!("unexpected retarget occurrence {other:?}"),
                };
                answer(&mut g, &r, Answer::Targets(choices));
            }
            RequestKind::ChooseTarget { .. } => panic!("copy retarget should ask for one occurrence at a time"),
            RequestKind::May { .. } => answer(&mut g, &r, Answer::Yes),
            RequestKind::Priority { .. } if !g.state.stack.is_empty() => answer(&mut g, &r, Answer::Pass),
            RequestKind::Priority { .. } => break,
            other => panic!("unexpected {other:?}"),
        }
        r = next(&mut g);
    }
    assert!(!g.state.battlefield().contains(&yours), "the copy's 2 went to the new first target");
    assert!(!g.state.battlefield().contains(&theirs), "the original's 2 to its own");
    assert_eq!(g.state.player(THEM).life, life - 2);
}
