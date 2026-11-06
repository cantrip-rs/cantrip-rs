//! CR 701.19a-c: shields and static regeneration, and prohibitions on both.
use crate::{Cx, Game, Setup};
use cantrip_api::Game as _;
use cantrip_api::*;
use cantrip_state::{Frame, FrameKind};

const YOU: PlayerId = PlayerId(0);
const THEM: PlayerId = PlayerId(1);

fn creatures(q: &dyn Query, _: &Asker, id: ObjectId) -> bool {
    q.chars(id).is(CardType::Creature)
}

fn destroyed(_: &dyn Query, asker: &Asker, proposal: &Proposal) -> bool {
    matches!(proposal, Proposal::ZoneChange(z) if z.object == asker.source && z.destruction()
        && z.from.is(ZoneKind::Battlefield) && z.to.is(ZoneKind::Graveyard))
}

fn regenerate(_: &dyn Query, _: &Asker, proposal: &mut Proposal) {
    if let Proposal::ZoneChange(z) = proposal {
        // The core enforces prohibitions, even if a replacement requests it.
        z.regenerated = true;
    }
}

static LAND: CardDef = CardDef { name: "Regen test land", types: TypeSet::of(&[CardType::Land]), ..CardDef::BLANK };
static BEAR: CardDef = CardDef {
    name: "Regen test bear", types: TypeSet::of(&[CardType::Creature]), power: Some(2), toughness: Some(2), ..CardDef::BLANK
};
static BAN: CardDef = CardDef {
    name: "Regen test ban", types: TypeSet::of(&[CardType::Enchantment]), regeneration_bans: &[creatures], ..CardDef::BLANK
};
static EMBLEM: CardDef = CardDef { name: "Regen test emblem", regeneration_bans: &[creatures], ..CardDef::BLANK };
static CLERIC: CardDef = CardDef {
    name: "Regen test cleric", types: TypeSet::of(&[CardType::Creature]), power: Some(2), toughness: Some(2),
    replacements: &[replacement(ReplacementKind::ZoneChange, destroyed, regenerate, "regenerate it instead")], ..CardDef::BLANK
};
static GRANTER: CardDef = CardDef {
    name: "Regen test grant", types: TypeSet::of(&[CardType::Enchantment]),
    delayed: &[granted_regeneration_ban(creatures, "Creatures can't be regenerated.")], ..CardDef::BLANK
};

fn game(answers: Vec<Answer>) -> Game {
    let table = Box::leak(Box::new(CardTable::new([&LAND, &BEAR, &BAN, &EMBLEM, &CLERIC, &GRANTER])));
    let deck = vec![table.id_of(&LAND); 30];
    let mut game = Game::new(table, &Setup { decks: vec![deck.clone(), deck], ..Default::default() });
    game.verify = true;
    game.state.frame_push(Frame::new(FrameKind::Settle));
    let at = game.state.agenda.len() - 1;
    game.state.update_frame(at, |f| f.answers = answers.into_iter().map(|answer| (None, answer)).collect());
    game
}

fn put(game: &mut Game, def: &'static CardDef, zone: Zone) -> ObjectId {
    let id = game.state.push_obj(Obj::new(game.cards.id_of(def), YOU, zone));
    game.state.zone_push(zone, id);
    id
}

fn cx(game: &mut Game) -> Cx<'_> {
    let frame = game.state.agenda.len() - 1;
    Cx { g: game, frame, cursor: 0, mana_source: None, entering: None, lineage: Vec::new(), damage: None, asked_at: None, doer: None, producing: None }
}

fn effect(game: &mut Game, source: ObjectId, target: ObjectId, kind: EffectKind) {
    cx(game).add_effect(source, target, kind, Duration::EndOfTurn);
}

fn shields(game: &Game, id: ObjectId) -> usize {
    game.state.effects.iter().filter(|e| e.affected == id && e.kind == EffectKind::RegenerationShield).count()
}

fn damaged_attacker(game: &mut Game, def: &'static CardDef) -> ObjectId {
    let id = put(game, def, Zone::BATTLEFIELD);
    game.state.update_obj(id, |o| { o.damage = 1; o.deathtouched = true; });
    game.state.set_attackers(vec![Attacker::new(id, Target::Player(THEM), THEM, TypeSet::NONE)]);
    id
}

fn assert_regenerated(game: &Game, id: ObjectId) {
    assert!(game.state.battlefield().contains(&id));
    let obj = game.state.obj(id);
    assert!(obj.tapped);
    assert_eq!(obj.damage, 0);
    assert!(!obj.deathtouched);
    assert!(!game.state.attackers.iter().any(|a| a.id == id));
    assert!(!game.state.blocks.iter().any(|b| b.blocker == id));
}

#[test]
fn a_normal_shield_still_heals_taps_and_removes_from_combat() {
    let mut game = game(vec![]);
    let id = damaged_attacker(&mut game, &BEAR);
    effect(&mut game, id, id, EffectKind::RegenerationShield);
    assert!(cx(&mut game).destroy_all(&[id]).is_empty());
    assert_regenerated(&game, id);
    assert_eq!(shields(&game, id), 0);
}

#[test]
fn a_resolved_ban_holds_after_the_target_loses_abilities_or_the_source_leaves() {
    let mut game = game(vec![]);
    let source = put(&mut game, &LAND, Zone::BATTLEFIELD);
    let id = put(&mut game, &BEAR, Zone::BATTLEFIELD);
    effect(&mut game, id, id, EffectKind::RegenerationShield);
    effect(&mut game, source, id, EffectKind::CantBeRegenerated);
    effect(&mut game, id, id, EffectKind::LoseAllAbilities);
    cx(&mut game).exile_all(&[source]);
    assert_eq!(cx(&mut game).destroy_all(&[id]).len(), 1);
    assert!(!game.state.battlefield().contains(&id));
}

#[test]
fn a_ban_allows_shield_creation_and_a_shield_works_after_the_ban_ends() {
    let mut game = game(vec![]);
    let id = put(&mut game, &BEAR, Zone::BATTLEFIELD);
    effect(&mut game, id, id, EffectKind::CantBeRegenerated);
    effect(&mut game, id, id, EffectKind::RegenerationShield);
    assert_eq!(shields(&game, id), 1);
    assert!(!game.state.obj(id).tapped, "creating a shield does not regenerate");
    game.state.effects_retain(|e| e.kind != EffectKind::CantBeRegenerated);
    assert!(cx(&mut game).destroy_all(&[id]).is_empty());
    assert_regenerated(&game, id);
}

#[test]
fn static_bans_apply_to_the_named_objects_even_after_those_lose_abilities() {
    let mut game = game(vec![]);
    put(&mut game, &BAN, Zone::BATTLEFIELD);
    let bear = put(&mut game, &BEAR, Zone::BATTLEFIELD);
    let land = put(&mut game, &LAND, Zone::BATTLEFIELD);
    effect(&mut game, bear, bear, EffectKind::LoseAllAbilities);
    effect(&mut game, bear, bear, EffectKind::RegenerationShield);
    effect(&mut game, land, land, EffectKind::RegenerationShield);
    assert_eq!(cx(&mut game).destroy_all(&[bear]).len(), 1);
    assert!(cx(&mut game).destroy_all(&[land]).is_empty(), "a noncreature is not banned");
    assert_regenerated(&game, land);
}

#[test]
fn losing_the_static_sources_abilities_or_removing_it_lifts_its_ban() {
    for remove in [false, true] {
        let mut game = game(vec![]);
        let source = put(&mut game, &BAN, Zone::BATTLEFIELD);
        let id = put(&mut game, &BEAR, Zone::BATTLEFIELD);
        effect(&mut game, id, id, EffectKind::RegenerationShield);
        if remove { cx(&mut game).exile_all(&[source]); }
        else { effect(&mut game, source, source, EffectKind::LoseAllAbilities); }
        assert!(cx(&mut game).destroy_all(&[id]).is_empty());
        assert_regenerated(&game, id);
    }
}

#[test]
fn a_ban_source_destroyed_simultaneously_still_bans_regeneration_for_that_event() {
    let mut game = game(vec![]);
    let source = put(&mut game, &BAN, Zone::BATTLEFIELD);
    let id = put(&mut game, &BEAR, Zone::BATTLEFIELD);
    effect(&mut game, id, id, EffectKind::RegenerationShield);
    assert_eq!(cx(&mut game).destroy_all(&[source, id]).len(), 2);
}

#[test]
fn an_emblem_can_forbid_regeneration_too() {
    let mut game = game(vec![]);
    put(&mut game, &EMBLEM, Zone::COMMAND);
    let id = put(&mut game, &BEAR, Zone::BATTLEFIELD);
    effect(&mut game, id, id, EffectKind::RegenerationShield);
    assert_eq!(cx(&mut game).destroy_all(&[id]).len(), 1);
}

#[test]
fn a_static_regeneration_replacement_works_without_a_shield() {
    let mut game = game(vec![]);
    let id = damaged_attacker(&mut game, &CLERIC);
    assert_eq!(shields(&game, id), 0);
    assert!(cx(&mut game).destroy_all(&[id]).is_empty());
    assert_regenerated(&game, id);
}

#[test]
fn choosing_static_regeneration_does_not_consume_an_unrelated_shield() {
    let mut game = game(vec![Answer::Index(1)]);
    let id = damaged_attacker(&mut game, &CLERIC);
    effect(&mut game, id, id, EffectKind::RegenerationShield);
    assert!(cx(&mut game).destroy_all(&[id]).is_empty());
    assert_regenerated(&game, id);
    assert_eq!(shields(&game, id), 1);
    let frame = game.state.agenda.last().unwrap();
    assert!(frame.pending.is_none(), "the replacement choice was answered");
}

#[test]
fn a_prohibition_also_defeats_static_regeneration_and_one_shot_no_regen_destruction() {
    for lasting in [false, true] {
        let mut game = game(vec![]);
        let id = put(&mut game, &CLERIC, Zone::BATTLEFIELD);
        if lasting {
            effect(&mut game, id, id, EffectKind::CantBeRegenerated);
            assert_eq!(cx(&mut game).destroy_all(&[id]).len(), 1);
        } else {
            assert_eq!(cx(&mut game).destroy_all_no_regen(&[id]).len(), 1);
        }
    }
}

#[test]
fn a_shield_counter_can_still_save_a_permanent_whose_static_regeneration_is_banned() {
    let mut game = game(vec![Answer::Index(1)]);
    let id = put(&mut game, &CLERIC, Zone::BATTLEFIELD);
    game.state.update_obj(id, |o| o.damage = 1);
    effect(&mut game, id, id, EffectKind::CantBeRegenerated);
    crate::counters::put(&mut cx(&mut game), id, CounterKind::Shield, 1, YOU, crate::counters::How::Effect);
    assert!(cx(&mut game).destroy_all(&[id]).is_empty());
    assert!(game.state.battlefield().contains(&id));
    assert_eq!(game.state.obj(id).counters(CounterKind::Shield), 0);
    assert!(!game.state.obj(id).tapped, "a shield counter does not regenerate");
    assert_eq!(game.state.obj(id).damage, 1, "a shield counter does not heal");
    assert!(game.state.agenda.last().unwrap().pending.is_none());
}

#[test]
fn regeneration_bans_apply_to_lethal_damage_and_deathtouch_state_based_destruction() {
    for touch in [false, true] {
        let mut game = game(vec![]);
        let id = put(&mut game, &BEAR, Zone::BATTLEFIELD);
        game.state.update_obj(id, |o| { o.damage = if touch { 1 } else { 2 }; o.deathtouched = touch; });
        effect(&mut game, id, id, EffectKind::RegenerationShield);
        effect(&mut game, id, id, EffectKind::CantBeRegenerated);
        crate::settle::state_based_actions_alone(&mut cx(&mut game)).unwrap();
        assert!(!game.state.battlefield().contains(&id));
    }
}

fn grant_ban(game: &mut Game, holder: ObjectId) {
    let card = game.cards.id_of(&GRANTER);
    effect(game, holder, holder, EffectKind::AddAbility(AbilityRef::delayed(card, 0)));
}

#[test]
fn a_granted_ban_functions_from_its_holder_and_later_ability_loss_removes_it() {
    for lose in [false, true] {
        let mut game = game(vec![]);
        let holder = put(&mut game, &BEAR, Zone::BATTLEFIELD);
        let target = put(&mut game, &BEAR, Zone::BATTLEFIELD);
        grant_ban(&mut game, holder);
        if lose { effect(&mut game, holder, holder, EffectKind::LoseAllAbilities); }
        effect(&mut game, target, target, EffectKind::RegenerationShield);
        let destroyed = cx(&mut game).destroy_all(&[target]);
        assert_eq!(destroyed.is_empty(), lose);
        if lose { assert_regenerated(&game, target); }
    }
}

#[test]
fn a_ban_granted_after_ability_loss_functions_and_ends_when_its_holder_leaves() {
    let mut game = game(vec![]);
    let holder = put(&mut game, &BEAR, Zone::BATTLEFIELD);
    effect(&mut game, holder, holder, EffectKind::LoseAllAbilities);
    grant_ban(&mut game, holder);
    let target = put(&mut game, &BEAR, Zone::BATTLEFIELD);
    effect(&mut game, target, target, EffectKind::RegenerationShield);
    assert_eq!(cx(&mut game).destroy_all(&[target]).len(), 1);
    cx(&mut game).exile_all(&[holder]);
    let free = put(&mut game, &BEAR, Zone::BATTLEFIELD);
    effect(&mut game, free, free, EffectKind::RegenerationShield);
    assert!(cx(&mut game).destroy_all(&[free]).is_empty());
    assert_regenerated(&game, free);
}
