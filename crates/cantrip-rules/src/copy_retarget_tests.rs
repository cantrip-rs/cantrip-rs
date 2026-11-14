//! Copying and choosing new targets (CR 707.10c, 115.7d-e).
use crate::{actions::stack_item, ops, Cx, Game, Setup};
use cantrip_api::*;
use cantrip_state::{Frame, FrameKind};

const YOU: PlayerId = PlayerId(0);
const THEM: PlayerId = PlayerId(1);
fn creature(q: &dyn Query, _: &Asker, t: Target) -> bool {
    t.object().is_some_and(|id| q.is_on_battlefield(id) && q.chars(id).is(CardType::Creature))
}
fn your_creature(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    creature(q, asker, t) && t.object().is_some_and(|id| q.controller(id) == asker.controller)
}
fn nothing(_: &mut dyn cantrip_api::Game, _: &StackItem) -> Result<(), Suspend> { Ok(()) }
static CREATURE: CardDef = CardDef { name: "Retarget creature", types: TypeSet::of(&[CardType::Creature]), power: Some(1), toughness: Some(1), ..CardDef::BLANK };
fn itself(_: &dyn LayerQuery, source: ObjectId, object: ObjectId, _: &Chars) -> bool { source == object }
fn protection_red(_: &dyn LayerQuery, _: ObjectId, chars: &mut Chars) {
    chars.protection = chars.protection.with(Color::Red);
}
static PROTECTED_RED: CardDef = CardDef {
    name: "Retarget protection from red", statics: &[static_ability(Layer::Ability, itself, protection_red, "Protection from red")],
    ..CREATURE
};
static HEXPROOF: CardDef = CardDef { name: "Retarget hexproof creature", keywords: Keywords::of(&[Keyword::Hexproof]), ..CREATURE };
fn another_creature(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    creature(q, asker, t) && !q.chosen_targets().contains(&t)
}
fn creature_controlled_by_first_targets_controller(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    let Some(first) = q.chosen_targets().first().and_then(|target| target.object()) else { return false };
    creature(q, asker, t) && t.object().is_some_and(|id| q.controller(id) == q.controller(first))
}
static DEPENDENT_TARGETS: CardDef = CardDef { name: "Retarget dependent creatures", types: TypeSet::of(&[CardType::Instant]), spell: spell(&[target(creature), target(creature_controlled_by_first_targets_controller)], nothing), ..CardDef::BLANK };
static TWO_TARGETS: CardDef = CardDef { name: "Retarget two creatures", types: TypeSet::of(&[CardType::Instant]), spell: spell(&[target(creature), target(another_creature)], nothing), ..CardDef::BLANK };
static SPELL: CardDef = CardDef { name: "Retarget spell", types: TypeSet::of(&[CardType::Instant]), spell: spell(&[target(creature)], nothing), ..CardDef::BLANK };
fn creature_ability(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    let Target::Ability(serial) = t else { return false };
    q.state().stack.iter().any(|item| item.serial == serial && item.kind == StackKind::Activated
        && item.controller == asker.controller && q.chars(item.object).is(CardType::Creature))
}
static COPIER: CardDef = CardDef { name: "Retarget ability copier", abilities: &[ability(Cost::NONE, nothing, "Copy target ability").targets(&[target(creature_ability)])], ..CREATURE };
static BLUE_SPELL: CardDef = CardDef { name: "Retarget blue spell", colors: Some(ColorSet::of(Color::Blue)), ..SPELL };
static YOUR_SPELL: CardDef = CardDef { name: "Retarget your creature spell", spell: spell(&[target(your_creature)], nothing), ..SPELL };
fn game() -> Game {
    let table = Box::leak(Box::new(CardTable::new([&CREATURE, &HEXPROOF, &PROTECTED_RED, &SPELL, &BLUE_SPELL, &YOUR_SPELL, &COPIER, &TWO_TARGETS, &DEPENDENT_TARGETS])));
    let mut g = Game::new(table, &Setup { decks: vec![vec![], vec![]], ..Default::default() });
    g.state.frame_push(Frame::new(FrameKind::Resolve { serial: 1 }));
    g
}
fn put(g: &mut Game, card: &'static CardDef, player: PlayerId, zone: Zone) -> ObjectId {
    let id = g.state.push_obj(Obj::new(g.cards.id_of(card), player, zone));
    g.state.zone_push(zone, id);
    if !card.statics.is_empty() { g.state.see_statics(); }
    id
}
fn cx(g: &mut Game) -> Cx<'_> {
    Cx { g, frame: 0, cursor: 0, mana_source: None, entering: None, lineage: vec![], damage: None, asked_at: None, doer: None, producing: None }
}
fn on_stack(g: &mut Game, card: &'static CardDef, controller: PlayerId, target: ObjectId) -> ObjectId {
    let spell = put(g, card, controller, Zone::STACK);
    g.state.stack_push(stack_item(StackKind::Spell, spell, AbilityRef { card: g.cards.id_of(card), index: u8::MAX }, controller, &[Target::Object(target)]));
    spell
}
#[test]
fn copying_may_keep_an_illegal_original_target_even_when_other_targets_exist() {
    let mut g = game();
    let protected = put(&mut g, &HEXPROOF, YOU, Zone::BATTLEFIELD);
    let legal = put(&mut g, &CREATURE, YOU, Zone::BATTLEFIELD);
    let original = on_stack(&mut g, &SPELL, YOU, protected);
    let _ = ops::copy_spell(&mut cx(&mut g), original, THEM, true);
    let request = g.state.agenda[0].pending.as_ref().expect("the copier can choose");
    let RequestKind::ChooseTarget { candidates, least, optional, replacing, .. } = &request.kind else { panic!("unexpected prompt {:?}", request.kind) };
    assert!(candidates.contains(&Target::Object(legal)));
    assert!(!candidates.contains(&Target::Object(protected)), "an illegal original is not a new target candidate");
    assert_eq!(*replacing, Some((0, Target::Object(protected))));
    assert!(*optional && *least == 0, "keeping the illegal original must be an available answer");
}
#[test]
fn choosing_player_does_not_replace_the_spells_controller_for_target_legality() {
    let mut g = game();
    let current = put(&mut g, &CREATURE, YOU, Zone::BATTLEFIELD);
    let own = put(&mut g, &CREATURE, YOU, Zone::BATTLEFIELD);
    let theirs = put(&mut g, &CREATURE, THEM, Zone::BATTLEFIELD);
    let original = on_stack(&mut g, &YOUR_SPELL, YOU, current);
    let _ = ops::choose_new_targets(&mut cx(&mut g), original, THEM);
    let request = g.state.agenda[0].pending.as_ref().expect("the chooser has a choice");
    let RequestKind::ChooseTarget { candidates, .. } = &request.kind else { panic!("unexpected prompt") };
    assert!(candidates.contains(&Target::Object(own)));
    assert!(!candidates.contains(&Target::Object(theirs)));
}

#[test]
fn ability_copy_is_not_on_stack_or_an_option_while_targets_are_chosen() {
    // Echo, Perceptive Prodigy's ability can itself be copied by another copy effect.
    // Its copy cannot target itself (CR 115.5, 707.10c).
    let mut g = game();
    let source = put(&mut g, &COPIER, YOU, Zone::BATTLEFIELD);
    let other = stack_item(StackKind::Activated, source, AbilityRef { card: g.cards.id_of(&COPIER), index: 0 }, YOU, &[]);
    g.state.stack_push(other);
    let other_serial = g.state.stack[0].serial;
    let original = stack_item(StackKind::Activated, source, AbilityRef { card: g.cards.id_of(&COPIER), index: 0 }, YOU, &[Target::Ability(other_serial)]);
    g.state.stack_push(original);
    let original_serial = g.state.stack[1].serial;
    let _ = ops::copy_ability(&mut cx(&mut g), Target::Ability(original_serial), YOU, true);
    let request = g.state.agenda[0].pending.as_ref().expect("other abilities offer a choice");
    let RequestKind::ChooseTarget { candidates, .. } = &request.kind else { panic!("unexpected prompt") };
    assert!(candidates.contains(&Target::Ability(original_serial)));
    assert_eq!(g.state.stack.len(), 2, "a copy is put on the stack after its targets are chosen");
}

#[test]
fn copy_can_keep_or_change_an_illegal_original_target() {
    for change in [false, true] {
        let mut g = game();
        let protected = put(&mut g, &HEXPROOF, YOU, Zone::BATTLEFIELD);
        let legal = put(&mut g, &CREATURE, YOU, Zone::BATTLEFIELD);
        let original = on_stack(&mut g, &SPELL, YOU, protected);
        let answer = if change { vec![Target::Object(legal)] } else { vec![] };
        g.state.update_frame(0, |f| f.answers.push((Some(THEM), Answer::Targets(answer))));
        let copy = ops::copy_spell(&mut cx(&mut g), original, THEM, true).unwrap().unwrap();
        let copied = g.state.stack.iter().find(|i| i.object == copy).unwrap();
        assert_eq!(g.state.targets(copied), &[Target::Object(if change { legal } else { protected })]);
    }
}

fn two_target_spell(g: &mut Game, first: ObjectId, second: ObjectId) -> ObjectId {
    let id = put(g, &TWO_TARGETS, YOU, Zone::STACK);
    let mut item = stack_item(StackKind::Spell, id, AbilityRef { card: g.cards.id_of(&TWO_TARGETS), index: u8::MAX }, YOU, &[Target::Object(first), Target::Object(second)]);
    item.inline_spec_of[1] = 1;
    g.state.stack_push(item);
    id
}
#[test]
fn two_targets_can_swap_when_the_completed_set_is_legal() {
    let mut g = game();
    let first = put(&mut g, &CREATURE, YOU, Zone::BATTLEFIELD);
    let second = put(&mut g, &CREATURE, YOU, Zone::BATTLEFIELD);
    let spell = two_target_spell(&mut g, first, second);
    let mut original = g.state.stack[0];
    g.state.set_item_targets(&mut original, &[Target::Object(first), Target::Object(second)], &[0, 1], &[1, 2]);
    g.state.stack_set(0, original);
    g.state.update_frame(0, |f| {
        f.answers.push((Some(YOU), Answer::Targets(vec![Target::Object(second)])));
        f.answers.push((Some(YOU), Answer::Targets(vec![Target::Object(first)])));
    });
    let copy = ops::copy_spell(&mut cx(&mut g), spell, YOU, true).unwrap().unwrap();
    let copied = g.state.stack.iter().find(|i| i.object == copy).unwrap();
    assert_eq!(g.state.targets(copied), &[Target::Object(second), Target::Object(first)]);
    assert_eq!(g.state.shares(copied), &[1, 2], "retargeting preserves the original division");
}
#[test]
fn one_illegal_target_can_stay_while_another_changes() {
    let mut g = game();
    let first = put(&mut g, &HEXPROOF, YOU, Zone::BATTLEFIELD);
    let second = put(&mut g, &CREATURE, YOU, Zone::BATTLEFIELD);
    let third = put(&mut g, &CREATURE, YOU, Zone::BATTLEFIELD);
    let spell = two_target_spell(&mut g, first, second);
    g.state.update_frame(0, |f| {
        f.answers.push((Some(THEM), Answer::Targets(vec![])));
        f.answers.push((Some(THEM), Answer::Targets(vec![Target::Object(third)])));
    });
    let copy = ops::copy_spell(&mut cx(&mut g), spell, THEM, true).unwrap().unwrap();
    let copied = g.state.stack.iter().find(|i| i.object == copy).unwrap();
    assert_eq!(g.state.targets(copied), &[Target::Object(first), Target::Object(third)]);
}

#[test]
fn copy_can_keep_a_target_that_has_left_and_returned_as_a_new_object() {
    let mut g = game();
    let stale = put(&mut g, &CREATURE, YOU, Zone::BATTLEFIELD);
    let returned = put(&mut g, &CREATURE, YOU, Zone::BATTLEFIELD);
    let spell = on_stack(&mut g, &SPELL, YOU, stale);
    g.state.zone_remove(Zone::BATTLEFIELD, stale);
    g.state.update_obj(stale, |o| o.became = Some(returned));
    g.state.update_frame(0, |f| f.answers.push((Some(THEM), Answer::Targets(vec![]))));
    let copy = ops::copy_spell(&mut cx(&mut g), spell, THEM, true).unwrap().unwrap();
    let copied = g.state.stack.iter().find(|i| i.object == copy).unwrap();
    assert_eq!(g.state.targets(copied), &[Target::Object(stale)]);
    assert!(!crate::resolve::target_legal(g.cards, &g.state, &g.log, copied, 0));
}

#[test]
fn invalid_completed_assignment_is_rejected_then_can_be_retried() {
    let mut g = game();
    let first = put(&mut g, &CREATURE, YOU, Zone::BATTLEFIELD);
    let second = put(&mut g, &CREATURE, YOU, Zone::BATTLEFIELD);
    let _third = put(&mut g, &CREATURE, YOU, Zone::BATTLEFIELD);
    let spell = two_target_spell(&mut g, first, second);
    g.state.update_frame(0, |f| {
        f.answers.push((Some(YOU), Answer::Targets(vec![Target::Object(second)])));
        f.answers.push((Some(YOU), Answer::Targets(vec![])));
        f.answers.push((Some(YOU), Answer::Targets(vec![])));
        f.answers.push((Some(YOU), Answer::Targets(vec![])));
    });
    let copy = ops::copy_spell(&mut cx(&mut g), spell, YOU, true).unwrap().unwrap();
    let copied = g.state.stack.iter().find(|i| i.object == copy).unwrap();
    assert_eq!(g.state.targets(copied), &[Target::Object(first), Target::Object(second)]);
}

#[test]
fn an_ability_copy_may_keep_its_original_target() {
    let mut g = game();
    let source = put(&mut g, &COPIER, YOU, Zone::BATTLEFIELD);
    let other = stack_item(StackKind::Activated, source, AbilityRef { card: g.cards.id_of(&COPIER), index: 0 }, YOU, &[]);
    g.state.stack_push(other);
    let other_serial = g.state.stack[0].serial;
    let original = stack_item(StackKind::Activated, source, AbilityRef { card: g.cards.id_of(&COPIER), index: 0 }, YOU, &[Target::Ability(other_serial)]);
    g.state.stack_push(original);
    let original_serial = g.state.stack[1].serial;
    g.state.update_frame(0, |f| f.answers.push((Some(YOU), Answer::Targets(vec![]))));
    let copied_serial = ops::copy_ability(&mut cx(&mut g), Target::Ability(original_serial), YOU, true).unwrap().unwrap();
    let copied = g.state.stack.iter().find(|i| i.serial == copied_serial).unwrap();
    assert_ne!(copied_serial, original_serial);
    assert_eq!(g.state.targets(copied), &[Target::Ability(other_serial)]);
}

#[test]
fn a_red_copy_can_keep_a_protection_from_red_target_or_change_it() {
    // Fork's exception makes the copy red without changing the original spell.
    for change in [false, true] {
        let mut g = game();
        let protected = put(&mut g, &PROTECTED_RED, YOU, Zone::BATTLEFIELD);
        let legal = put(&mut g, &CREATURE, YOU, Zone::BATTLEFIELD);
        let original = on_stack(&mut g, &BLUE_SPELL, YOU, protected);
        assert!(crate::resolve::target_legal(g.cards, &g.state, &g.log, &g.state.stack[0], 0));
        let values = g.copiable_values(original).colored(ColorSet::of(Color::Red));
        let answer = if change { vec![Target::Object(legal)] } else { vec![] };
        g.state.update_frame(0, |f| f.answers.push((Some(THEM), Answer::Targets(answer))));
        let copy = ops::copy_spell_as(&mut cx(&mut g), original, THEM, true, Some(values)).unwrap().unwrap();
        let copied = g.state.stack.iter().find(|i| i.object == copy).unwrap();
        assert_eq!(g.chars(original).colors, ColorSet::of(Color::Blue));
        assert_eq!(g.chars(copy).colors, ColorSet::of(Color::Red));
        assert_eq!(g.state.targets(copied), &[Target::Object(if change { legal } else { protected })]);
        assert_eq!(crate::resolve::target_legal(g.cards, &g.state, &g.log, copied, 0), change);
    }
}

#[test]
fn a_later_target_is_offered_using_the_new_earlier_target() {
    // Goblin Welder's second target likewise depends on the first target's controller.
    let mut g = game();
    let own_first = put(&mut g, &CREATURE, YOU, Zone::BATTLEFIELD);
    let own_second = put(&mut g, &CREATURE, YOU, Zone::BATTLEFIELD);
    let their_first = put(&mut g, &CREATURE, THEM, Zone::BATTLEFIELD);
    let their_second = put(&mut g, &CREATURE, THEM, Zone::BATTLEFIELD);
    let id = put(&mut g, &DEPENDENT_TARGETS, YOU, Zone::STACK);
    let mut item = stack_item(StackKind::Spell, id, AbilityRef { card: g.cards.id_of(&DEPENDENT_TARGETS), index: u8::MAX }, YOU, &[Target::Object(own_first), Target::Object(own_second)]);
    item.inline_spec_of[1] = 1;
    g.state.stack_push(item);
    g.state.update_frame(0, |f| {
        f.answers.push((Some(YOU), Answer::Targets(vec![Target::Object(their_first)])));
        f.answers.push((Some(YOU), Answer::Targets(vec![Target::Object(their_second)])));
    });
    let copy = ops::copy_spell(&mut cx(&mut g), id, YOU, true).unwrap().unwrap();
    let copied = g.state.stack.iter().find(|i| i.object == copy).unwrap();
    assert_eq!(g.state.targets(copied), &[Target::Object(their_first), Target::Object(their_second)]);
}


// Glyph of Delusion and Goblin Welder need the same ordered target-availability check as retargeting.
fn has_targets(g: &mut Game, specs: &[TargetSpec]) -> bool {
    let source = put(g, &DEPENDENT_TARGETS, YOU, Zone::hand(YOU));
    crate::actions::slots_satisfiable(&cx(g), &Asker { source, controller: YOU, by: None, spell: true, instance: None }, specs)
}

#[test]
fn dependent_target_slots_are_available_only_with_a_complete_legal_announcement() {
    let mut g = game();
    assert!(!has_targets(&mut g, &[target(creature), target(creature_controlled_by_first_targets_controller)]));
    put(&mut g, &CREATURE, YOU, Zone::BATTLEFIELD);
    assert!(has_targets(&mut g, &[target(creature), target(creature_controlled_by_first_targets_controller)]));
    // A separate target instruction may target the first creature again; "another" cannot.
    assert!(!has_targets(&mut g, &[target(creature), target(another_creature)]));
    put(&mut g, &CREATURE, THEM, Zone::BATTLEFIELD);
    assert!(has_targets(&mut g, &[target(creature), target(another_creature)]));
}

fn other_creature_with_same_controller(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    creature_controlled_by_first_targets_controller(q, asker, t) && !q.chosen_targets().contains(&t)
}

#[test]
fn target_availability_tries_another_first_choice_when_the_first_one_has_no_partner() {
    let mut g = game();
    put(&mut g, &CREATURE, YOU, Zone::BATTLEFIELD);
    put(&mut g, &CREATURE, THEM, Zone::BATTLEFIELD);
    assert!(!has_targets(&mut g, &[target(creature), target(other_creature_with_same_controller)]));
    put(&mut g, &CREATURE, THEM, Zone::BATTLEFIELD);
    assert!(has_targets(&mut g, &[target(creature), target(other_creature_with_same_controller)]));
}

fn after_empty_slot(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    q.chosen_targets() == [Target::None] && creature(q, asker, t)
}

#[test]
fn target_availability_preserves_empty_optional_slots_and_unannounced_x() {
    let mut g = game();
    put(&mut g, &CREATURE, YOU, Zone::BATTLEFIELD);
    assert!(has_targets(&mut g, &[up_to(1, creature), target(after_empty_slot)]));
    // X can be zero; its slot is not a required minimum before X is announced.
    assert!(has_targets(&mut g, &[target(creature).counted(), target(creature)]));
}
