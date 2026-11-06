use crate::{Cx, Game, Setup};
use cantrip_api::*;
use cantrip_state::{Frame, FrameKind};

const YOU: PlayerId = PlayerId(0);
const THEM: PlayerId = PlayerId(1);
static LEADER: CardDef = CardDef { name: "Commander Rules Test", types: TypeSet::of(&[CardType::Creature]), power: Some(1), toughness: Some(1), ..CardDef::BLANK };
static MERGED_TOP: CardDef = CardDef { name: "Merged Top Test", types: TypeSet::of(&[CardType::Creature]), power: Some(1), toughness: Some(1), ..CardDef::BLANK };

fn hand_to_exile(_: &dyn Query, _: &Asker, proposal: &Proposal) -> bool {
    matches!(proposal, Proposal::ZoneChange(z) if z.to.is(ZoneKind::Hand))
}
fn exile_instead(_: &dyn Query, _: &Asker, proposal: &mut Proposal) {
    if let Proposal::ZoneChange(z) = proposal { z.to = Zone::exile(z.to.player); }
}
static HAND_EXILE: CardDef = CardDef {
    name: "Hand Exile Test",
    types: TypeSet::of(&[CardType::Enchantment]),
    replacements: &[replacement(ReplacementKind::ZoneChange, hand_to_exile, exile_instead, "exile it instead")],
    ..CardDef::BLANK
};

fn game() -> Game {
    let cards = Box::leak(Box::new(CardTable::new([&LEADER, &MERGED_TOP, &HAND_EXILE])));
    Game::new(cards, &Setup {
        decks: vec![Vec::new(), Vec::new()],
        commander: true,
        commanders: vec![vec![cards.id_of(&LEADER)], Vec::new()],
        ..Default::default()
    })
}

#[test]
fn commander_setup_sets_life_and_designates_a_physical_command_zone_card() {
    let game = game();
    assert_eq!(game.state.player(YOU).life, 40);
    assert_eq!(game.state.player(THEM).starting_life, 40);
    let commander = game.state.zone(Zone::COMMAND)[0];
    let object = game.state.obj(commander);
    assert!(object.commander);
    assert_eq!(object.commander_id, Some(commander));
}

#[test]
fn commander_damage_counts_only_actual_combat_damage_by_that_commander() {
    let mut game = game();
    let commander = game.state.zone(Zone::COMMAND)[0];
    let known = crate::ops::last_known_of(&mut Cx { g: &mut game, frame: usize::MAX, cursor: 0, mana_source: None, entering: None, lineage: Vec::new(), damage: None, asked_at: None, doer: None, producing: None }, commander);
    let known = game.state.last_known_push(known);
    let damage = |amount, combat| Event::Damage {
        source: commander,
        target: Target::Player(THEM),
        amount,
        kind: DamageKind::new(combat, ColorSet::NONE),
        controller: YOU,
        recipient_excess: 0,
        known,
    };
    for (serial, event) in [damage(12, true), damage(50, false), damage(9, true)].into_iter().enumerate() {
        game.log.push(Record { serial: serial as u32, turn: 1, step: Step::CombatDamage, active: YOU, cause: Cause::Turn, event });
    }
    assert_eq!(game.commander_damage(commander, THEM), 21);
    assert_eq!(game.commander_damage(commander, YOU), 0);
    game.state.update_player(THEM, |player| player.drew_from_empty = false);
    let cx = Cx { g: &mut game, frame: usize::MAX, cursor: 0, mana_source: None, entering: None, lineage: Vec::new(), damage: None, asked_at: None, doer: None, producing: None };
    assert_eq!(crate::ops::loses_to_state_based_actions(&cx, THEM), Some(LossReason::CommanderDamage));
}

#[test]
fn ordinary_outside_game_access_is_blocked_but_companion_special_action_remains_available() {
    let table = Box::leak(Box::new(CardTable::new([&LEADER])));
    let mut game = Game::new(table, &Setup {
        decks: vec![Vec::new(), Vec::new()],
        sideboards: vec![vec![table.id_of(&LEADER)], Vec::new()],
        commander: true,
        commanders: vec![vec![table.id_of(&LEADER)], Vec::new()],
        ..Default::default()
    });
    let sideboard = game.state.zone(Zone::sideboard(YOU))[0];
    assert!(Query::outside_the_game(&game, YOU).is_empty());
    let moved = crate::ops::bring_into_hand(&mut Cx { g: &mut game, frame: usize::MAX, cursor: 0, mana_source: None, entering: None, lineage: Vec::new(), damage: None, asked_at: None, doer: None, producing: None }, &[sideboard]);
    assert!(moved.is_empty());
    assert_eq!(game.state.obj(sideboard).zone, Zone::sideboard(YOU));
    let moved = crate::ops::bring_companion_into_hand(&mut Cx { g: &mut game, frame: usize::MAX, cursor: 0, mana_source: None, entering: None, lineage: Vec::new(), damage: None, asked_at: None, doer: None, producing: None }, sideboard);
    assert_eq!(moved.len(), 1);
    assert_eq!(game.state.obj(moved[0]).zone, Zone::hand(YOU));
}

#[test]
fn spell_copies_do_not_inherit_commander_identity() {
    let mut game = game();
    let commander = game.state.zone(Zone::COMMAND)[0];
    let spell = game.state.push_obj(Obj::new(game.state.obj(commander).card, YOU, Zone::STACK));
    game.state.update_obj(spell, |obj| {
        obj.commander = true;
        obj.commander_id = Some(commander);
    });
    game.state.zone_push(Zone::STACK, spell);
    game.state.stack_push(crate::actions::stack_item(StackKind::Spell, spell, AbilityRef { card: game.state.obj(spell).card, index: 0 }, YOU, &[]));
    let copy = crate::ops::copy_spell(&mut Cx { g: &mut game, frame: usize::MAX, cursor: 0, mana_source: None, entering: None, lineage: Vec::new(), damage: None, asked_at: None, doer: None, producing: None }, spell, YOU, false)
        .expect("copy does not ask").expect("copy created");
    assert!(game.state.obj(spell).commander);
    assert!(!game.state.obj(copy).commander);
    assert_eq!(game.state.obj(copy).commander_id, None);
}

fn merged_commander_game(answer: Answer) -> (Game, ObjectId, ObjectId) {
    let mut game = game();
    game.state.frame_push(Frame::new(FrameKind::Priority));
    let commander = game.state.zone(Zone::COMMAND)[0];
    game.state.zone_remove(Zone::COMMAND, commander);
    game.state.update_obj(commander, |obj| obj.zone = Zone::GONE);
    let top_card = game.cards.id_of(&MERGED_TOP);
    let permanent = game.state.push_obj(Obj::new(top_card, YOU, Zone::BATTLEFIELD));
    game.state.zone_push(Zone::BATTLEFIELD, permanent);
    game.state.set_merge(permanent, cantrip_state::Merge {
        own_face: top_card,
        own_token: false,
        own_commander_id: None,
        own_face_down: None,
        parts: vec![cantrip_state::MergedPart {
            card: game.cards.id_of(&LEADER), face: game.cards.id_of(&LEADER), token: false,
            commander_id: Some(commander), copy: None, on_top: false, timestamp: 1, prior: None, melded: false,
        }],
    });
    game.state.update_obj(permanent, |obj| obj.commander = true);
    let replacer = game.state.push_obj(Obj::new(game.cards.id_of(&HAND_EXILE), THEM, Zone::BATTLEFIELD));
    game.state.zone_push(Zone::BATTLEFIELD, replacer);
    let frame = 0;
    game.state.update_frame(frame, |f| f.answers = vec![(Some(YOU), Answer::Index(0)), (Some(YOU), answer)]);
    (game, permanent, commander)
}

#[test]
fn merged_commander_replacement_orders_per_component_and_does_not_misroute_the_top() {
    let (mut game, permanent, commander) = merged_commander_game(Answer::Yes);
    let frame = 0;
    let result = crate::ops::move_to(&mut Cx { g: &mut game, frame, cursor: 0, mana_source: None, entering: None, lineage: Vec::new(), damage: None, asked_at: None, doer: None, producing: None }, permanent, Zone::hand(YOU));
    assert_ne!(result, permanent);
    assert_eq!(game.state.zone(Zone::COMMAND).len(), 1, "the command zone contains exactly the returned physical commander");
    let command_piece = game.state.zone(Zone::COMMAND)[0];
    assert_eq!(game.state.obj(command_piece).card, game.cards.id_of(&LEADER));
    assert_eq!(game.state.obj(command_piece).commander_id, Some(commander));
    let exiled_top = game.state.zone(Zone::exile(YOU)).iter().copied().find(|&id| id != permanent).expect("the noncommander top followed the hand-to-exile replacement");
    assert_eq!(game.state.obj(exiled_top).card, game.cards.id_of(&MERGED_TOP));

    let (mut game, permanent, commander) = merged_commander_game(Answer::No);
    let frame = 0;
    crate::ops::move_to(&mut Cx { g: &mut game, frame, cursor: 0, mana_source: None, entering: None, lineage: Vec::new(), damage: None, asked_at: None, doer: None, producing: None }, permanent, Zone::hand(YOU));
    assert!(game.state.zone(Zone::COMMAND).is_empty(), "declining sends the physical commander with the rest of the merged permanent");
    assert_eq!(game.state.zone(Zone::exile(YOU)).len(), 2, "declining lets the competing replacement exile both components");
    assert!(game.state.zone(Zone::exile(YOU)).iter().any(|&id| game.state.obj(id).commander_id == Some(commander)));
}

#[test]
fn commander_identity_under_a_merged_top_is_snapshotted_for_damage_and_component_splits() {
    let mut game = game();
    let commander = game.state.zone(Zone::COMMAND)[0];
    let card = game.state.obj(commander).card;
    let permanent = game.state.push_obj(Obj::new(card, YOU, Zone::BATTLEFIELD));
    game.state.zone_push(Zone::BATTLEFIELD, permanent);
    let before = crate::ops::last_known_of(&mut Cx { g: &mut game, frame: usize::MAX, cursor: 0, mana_source: None, entering: None, lineage: Vec::new(), damage: None, asked_at: None, doer: None, producing: None }, permanent);
    let before_index = game.state.last_known_push(before);
    game.log.push(Record {
        serial: 0, turn: 1, step: Step::CombatDamage, active: YOU, cause: Cause::Turn,
        event: Event::Damage { source: permanent, target: Target::Player(THEM), amount: 10,
            kind: DamageKind::new(true, ColorSet::NONE), controller: YOU, recipient_excess: 0, known: before_index },
    });
    let merge = cantrip_state::Merge {
        own_face: card,
        own_token: false,
        own_commander_id: None,
        own_face_down: None,
        parts: vec![cantrip_state::MergedPart { card, face: card, token: false, commander_id: Some(commander), copy: None, on_top: false, timestamp: 1, prior: None, melded: false }],
    };
    game.state.set_merge(permanent, merge);
    game.state.update_obj(permanent, |obj| obj.commander = true);
    let components = crate::merge::components(&Cx { g: &mut game, frame: usize::MAX, cursor: 0, mana_source: None, entering: None, lineage: Vec::new(), damage: None, asked_at: None, doer: None, producing: None }, permanent)
        .expect("merged components");
    assert_eq!(components.iter().find(|c| !c.top).unwrap().commander_id, Some(commander));
    let known = crate::ops::last_known_of(&mut Cx { g: &mut game, frame: usize::MAX, cursor: 0, mana_source: None, entering: None, lineage: Vec::new(), damage: None, asked_at: None, doer: None, producing: None }, permanent);
    assert!(known.commander_ids.contains(&commander));
    let after_index = game.state.last_known_push(known);
    game.log.push(Record {
        serial: 1, turn: 1, step: Step::CombatDamage, active: YOU, cause: Cause::Turn,
        event: Event::Damage { source: permanent, target: Target::Player(THEM), amount: 5,
            kind: DamageKind::new(true, ColorSet::NONE), controller: YOU, recipient_excess: 0, known: after_index },
    });
    assert_eq!(game.commander_damage(commander, THEM), 5, "damage before the component became a commander does not count retroactively");
}
