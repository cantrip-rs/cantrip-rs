//! Relationship changes preserve status, trigger identity, banding and first-strike provenance.
use super::*;
use cantrip_api::Game as Rules;
use std::hash::{Hash, Hasher};
const YOU: PlayerId = PlayerId(0);
const THEM: PlayerId = PlayerId(1);
static CREATURE: CardDef = CardDef {
    name: "Combat change creature", types: TypeSet::of(&[CardType::Creature]),
    power: Some(2), toughness: Some(20), ..CardDef::BLANK
};
static STRIKER: CardDef = CardDef { name: "Combat change first striker", keywords: Keywords::of(&[Keyword::FirstStrike]), ..CREATURE };
static BATTLE: CardDef = CardDef { name: "Combat change creature battle", types: TypeSet::of(&[CardType::Creature, CardType::Battle]), ..CREATURE };
static FLYER: CardDef = CardDef { name:"Combat change flyer", keywords:Keywords::of(&[Keyword::Flying]), ..CREATURE };
static MENACE: CardDef = CardDef { name:"Combat change menace", keywords:Keywords::of(&[Keyword::Menace]), ..CREATURE };
static LONE: CardDef = CardDef { name:"Combat change single-block limit", keywords:Keywords::of(&[Keyword::CantBeBlockedByMoreThanOne]), ..CREATURE };
static RED: CardDef = CardDef { name:"Combat change red creature", colors:Some(ColorSet::of(Color::Red)), ..CREATURE };
static PROTECTED: CardDef = CardDef { name:"Combat change red protection", protection:ColorSet::of(Color::Red), ..CREATURE };
static MULTI: CardDef = CardDef { name:"Combat change multi blocker", abilities:&[block_allowance(1,"Can block an additional creature.")], ..CREATURE };
fn any_blocker(_: &dyn Query, _: &Asker, _: ObjectId) -> bool {true}
fn everyone(_: &dyn Query, _: &Asker, _: ObjectId, _: ObjectId) -> bool {true}
static CAP: CardDef = CardDef { name:"Combat change group cap", combat_groups:&[
    CombatGroupRule {role:CombatRole::Blocker,applies:any_blocker,counted:everyone,others_only:false,at_least:0,at_most:2}
], ..CardDef::BLANK };
fn blocks(_: &dyn Query, source: ObjectId, e: &Event) -> Option<Fired> {
    matches!(e, Event::Blocks { blocker, declared: true, .. } if *blocker == source).then_some(Fired::PLAIN).flatten()
}
fn each_block(_: &dyn Query, source: ObjectId, e: &Event) -> Option<Fired> {
    matches!(e, Event::Blocks { blocker, .. } if *blocker == source).then_some(Fired::PLAIN).flatten()
}
fn nothing(_: &mut dyn Rules, _: &StackItem) -> Result<(), Suspend> { Ok(()) }
static WITNESS: CardDef = CardDef {
    name: "Combat change trigger witness", abilities: &[
        triggered(when(EventMask::BLOCKS, blocks), nothing, "Whenever this creature blocks."),
        triggered(when(EventMask::BLOCKS, each_block), nothing, "Whenever this creature blocks a creature."),
    ], ..CREATURE
};
fn cx(g: &mut Game) -> Cx<'_> {
    Cx { g, frame: 0, cursor: 0, mana_source: None, entering: None, lineage: Vec::new(), damage: None, asked_at: None, doer: None, producing: None }
}
fn game() -> Game {
    let table = Box::leak(Box::new(CardTable::new([&CREATURE, &STRIKER, &WITNESS, &BATTLE,&FLYER,&MENACE,&LONE,&RED,&PROTECTED,&MULTI,&CAP])));
    let deck = vec![table.id_of(&CREATURE); 20];
    let mut g = Game::new(table, &Setup { decks: vec![deck; 2], ..Default::default() });
    while !g.state.agenda.is_empty() { g.state.frame_remove(0); }
    g.state.frame_push(cantrip_state::Frame::new(cantrip_state::FrameKind::Settle));
    g.state.update_turn(|t| { t.active = YOU; t.number = 1; t.set_step(Step::DeclareBlockers); });
    cx(&mut g).record(Event::StepBegan { step: Step::BeginCombat });
    g
}
fn put(g: &mut Game, def: &'static CardDef, player: PlayerId) -> ObjectId {
    let id = g.state.push_obj(Obj::new(g.cards.id_of(def), player, Zone::BATTLEFIELD));
    g.state.zone_push(Zone::BATTLEFIELD, id); id
}
fn attack(g: &mut Game) -> ObjectId {
    attack_as(g,&CREATURE)
}
fn attack_as(g: &mut Game, def: &'static CardDef) -> ObjectId {
    let id = put(g, def, YOU);
    let mut a = g.state.attackers.clone();
    a.push(Attacker::new(id, Target::Player(THEM), THEM, TypeSet::NONE));
    g.state.set_attackers(a); id
}
fn update_attackers(g: &mut Game, update: impl FnOnce(&mut [Attacker])) {
    let mut attackers = g.state.attackers.clone(); update(&mut attackers); g.state.set_attackers(attackers);
}
fn edges(g: &Game) -> Vec<(ObjectId, ObjectId, bool)> {
    g.log.iter().filter_map(|r| match r.event {
        Event::Blocks { blocker, attacker, declared } => Some((blocker, attacker, declared)), _ => None,
    }).collect()
}
fn damage(g: &mut Game) {
    g.state.update_turn(|t| t.set_step(Step::CombatDamage));
    let frame = g.state.agenda.len();
    g.state.frame_push(cantrip_state::Frame::new(cantrip_state::FrameKind::BeginStep));
    let mut c = cx(g); c.frame = frame;
    crate::turn::begin_step(&mut c).expect("forced one-to-one position needs no damage division");
}

#[test]
fn an_effect_blocks_an_unblocked_attacker_and_emits_each_trigger_kind_once() {
    let mut g = game(); let a = attack(&mut g); let b = put(&mut g, &WITNESS, THEM);
    cx(&mut g).change_blocks(&[], &[], &[(b,a), (b,a)]);
    assert_eq!(edges(&g), [(b,a,true)]);
    assert_eq!(g.state.blocks.len(), 1);
    assert!(g.is_blocked(a)); assert!(g.is_blocking(b));
    assert_eq!(g.state.triggers.len(), 2);
    assert_eq!(g.log.iter().filter(|r| matches!(r.event, Event::BecameBlocked { attacker } if attacker == a)).count(), 1);
}

#[test]
fn a_new_multi_blocker_has_one_generic_and_two_per_creature_triggers() {
    let mut g = game(); let a = attack(&mut g); let other = attack(&mut g); let b = put(&mut g, &WITNESS, THEM);
    cx(&mut g).change_blocks(&[], &[], &[(b,a), (b,other)]);
    assert_eq!(edges(&g), [(b,a,true), (b,other,false)]);
    assert_eq!(g.state.triggers.len(), 3);
}

#[test]
fn moving_a_blocking_relationship_does_not_repeat_the_generic_blocks_trigger() {
    let mut g = game(); let a = attack(&mut g); let other = attack(&mut g); let b = put(&mut g, &WITNESS, THEM);
    g.state.set_blocks(vec![Block { blocker:b, attacker:a, first_strike:false }]);
    update_attackers(&mut g, |a| { for a in a { a.blocked = true; } });
    cx(&mut g).change_blocks(&[], &[(b,a)], &[(b,other)]);
    assert_eq!(edges(&g), [(b,other,false)]);
    assert!(g.is_blocked(a)); assert!(g.is_blocking(b));
    assert_eq!(g.state.triggers.len(), 1);
    assert!(!g.log.iter().any(|r| matches!(r.event, Event::BecameBlocked { .. } | Event::Unblocked { .. })));
}

#[test]
fn removing_and_reassigning_a_blocker_repeats_its_block_events_even_for_the_same_attacker() {
    let mut g = game(); let a = attack(&mut g); let b = put(&mut g, &WITNESS, THEM);
    g.state.set_blocks(vec![Block { blocker:b, attacker:a, first_strike:false }]);
    update_attackers(&mut g, |a| a[0].blocked = true);
    cx(&mut g).change_blocks(&[b], &[], &[(b,a)]);
    assert_eq!(edges(&g), [(b,a,true)]); assert_eq!(g.state.triggers.len(), 2);
    assert!(!g.log.iter().any(|r| matches!(r.event, Event::BecameBlocked { .. } | Event::Unblocked { .. })));
}

#[test]
fn retaining_a_shared_pair_is_not_a_new_block() {
    let mut g = game(); let a = attack(&mut g); let b = put(&mut g, &WITNESS, THEM);
    g.state.set_blocks(vec![Block { blocker:b, attacker:a, first_strike:false }]);
    update_attackers(&mut g, |a| a[0].blocked = true);
    cx(&mut g).change_blocks(&[], &[(b,a)], &[(b,a)]);
    assert!(edges(&g).is_empty()); assert!(g.state.triggers.is_empty());
}

#[test]
fn ending_the_last_pair_preserves_independent_blocking_and_blocked_status() {
    let mut g = game(); let a = attack(&mut g); let b = put(&mut g, &CREATURE, THEM);
    g.state.set_blocks(vec![Block { blocker:b, attacker:a, first_strike:false }]);
    update_attackers(&mut g, |a| a[0].blocked = true);
    cx(&mut g).change_blocks(&[], &[(b,a)], &[]);
    assert!(g.state.blocks.is_empty()); assert!(g.is_blocking(b)); assert!(g.is_blocked(a));
}

#[test]
fn assigning_a_blocker_to_one_band_member_blocks_each_member_once() {
    let mut g = game(); let a = attack(&mut g); let other = attack(&mut g); let b = put(&mut g, &WITNESS, THEM);
    update_attackers(&mut g, |a| for a in a { a.band = 1; });
    cx(&mut g).change_blocks(&[], &[], &[(b,a), (b,other)]);
    assert_eq!(edges(&g), [(b,a,true), (b,other,false)]);
    assert_eq!(g.state.triggers.len(), 3); assert!(g.state.attackers.iter().all(|a| a.blocked));
}

#[test]
fn an_effect_can_assign_a_tapped_blocker_without_a_declaration() {
    let mut g = game(); let a = attack(&mut g); let b = put(&mut g, &CREATURE, THEM);
    g.state.update_obj(b, |o| o.tapped = true);
    cx(&mut g).change_blocks(&[], &[], &[(b,a)]);
    assert_eq!(g.state.blocks.len(), 1); assert!(g.state.obj(b).tapped);
}

#[test]
fn creatures_outside_the_battlefield_and_creature_battles_cannot_become_blockers() {
    let mut g = game(); let a = attack(&mut g); let b = put(&mut g, &CREATURE, THEM); let battle = put(&mut g, &BATTLE, THEM);
    cx(&mut g).return_to_hand(&[b]);
    cx(&mut g).change_blocks(&[], &[], &[(b,a), (battle,a), (a,a)]);
    assert!(g.state.blocks.is_empty()); assert!(edges(&g).is_empty());
}

#[test]
fn reassigning_a_surviving_first_striker_after_its_attacker_left_preserves_damage_eligibility() {
    for double in [false, true] {
        let mut g = game(); let a = attack(&mut g); let other = attack(&mut g); let b = put(&mut g, &STRIKER, THEM);
        g.state.set_blocks(vec![Block { blocker:b, attacker:a, first_strike:false }]);
        update_attackers(&mut g, |a| a[0].blocked = true);
        damage(&mut g);
        assert_eq!(g.state.turn.damage_round, 1); assert_eq!(g.state.obj(a).damage, 2);
        assert!(g.log.iter().any(|r| matches!(r.event, Event::CombatFirstStrike { creature } if creature == b)));
        cx(&mut g).remove_from_combat(a);
        assert!(g.is_blocking(b)); assert!(g.state.blocks.is_empty());
        cx(&mut g).add_effect(b,b,EffectKind::LoseAllAbilities,Duration::EndOfTurn);
        if double { cx(&mut g).add_effect(b,b,EffectKind::AddKeywords(Keywords::of(&[Keyword::DoubleStrike])),Duration::EndOfTurn); }
        cx(&mut g).change_blocks(&[b], &[], &[(b,other)]);
        assert!(g.state.blocks[0].first_strike);
        damage(&mut g);
        assert_eq!(g.state.obj(other).damage, if double { 2 } else { 0 });
    }
}

#[test]
fn first_strike_history_does_not_leak_into_another_combat() {
    let mut g = game(); let a = attack(&mut g); let b = put(&mut g, &STRIKER, THEM);
    cx(&mut g).record(Event::CombatFirstStrike { creature:b });
    cx(&mut g).record(Event::StepBegan { step:Step::BeginCombat });
    cx(&mut g).change_blocks(&[], &[], &[(b,a)]);
    assert!(!g.state.blocks[0].first_strike);
}

fn hash(g: &Game) -> u64 { let mut h=std::collections::hash_map::DefaultHasher::new(); g.state.hash(&mut h); h.finish() }
#[test]
fn relationship_status_triggers_and_provenance_rollback_and_replay() {
    let mut g = game(); let a = attack(&mut g); let b = put(&mut g, &WITNESS, THEM);
    let mark = g.state.mark(); let log = g.log.len(); let before = hash(&g);
    cx(&mut g).change_blocks(&[], &[], &[(b,a)]);
    let after = hash(&g); let events: Vec<_> = g.log[log..].iter().map(|r| r.event).collect();
    g.state.rollback(mark); g.log.truncate(log); assert_eq!(hash(&g),before);
    cx(&mut g).change_blocks(&[], &[], &[(b,a)]);
    assert_eq!(hash(&g),after); assert_eq!(g.log[log..].iter().map(|r| r.event).collect::<Vec<_>>(),events);
}

fn set_pairs(g:&mut Game,pairs:&[(ObjectId,ObjectId)]) {
    g.state.set_blocks(pairs.iter().map(|&(blocker,attacker)|Block {blocker,attacker,first_strike:false}).collect());
    update_attackers(g,|a|for a in a {a.blocked=true;});
}
fn swap(g:&mut Game,first:ObjectId,second:ObjectId,attacking:bool) {
    cantrip_mechanics::combat::swap_blocks(&mut cx(g),Some(first),Some(second),attacking);
}
#[test]
fn attacker_exchange_moves_exclusive_edges_and_keeps_shared_ones() {
    let mut g=game();let a=attack(&mut g);let b=attack(&mut g);
    let x=put(&mut g,&WITNESS,THEM);let y=put(&mut g,&WITNESS,THEM);let shared=put(&mut g,&MULTI,THEM);
    set_pairs(&mut g,&[(x,a),(y,b),(shared,a),(shared,b)]);
    swap(&mut g,a,b,true);
    assert!(g.state.blocks.iter().any(|p|p.blocker==x&&p.attacker==b));
    assert!(g.state.blocks.iter().any(|p|p.blocker==y&&p.attacker==a));
    assert_eq!(g.state.blocks.iter().filter(|p|p.blocker==shared).count(),2);
    assert_eq!(edges(&g),[(x,b,false),(y,a,false)]);assert_eq!(g.state.triggers.len(),2);
}
#[test]
fn current_flying_and_reach_decide_whether_an_exchange_is_allowed() {
    let mut g=game();let a=attack_as(&mut g,&FLYER);let b=attack(&mut g);
    let x=put(&mut g,&FLYER,THEM);let y=put(&mut g,&CREATURE,THEM);
    set_pairs(&mut g,&[(x,a),(y,b)]);let before=g.state.blocks.clone();
    swap(&mut g,a,b,true);assert_eq!(g.state.blocks,before);
    cx(&mut g).add_effect(y,y,EffectKind::AddKeywords(Keywords::of(&[Keyword::Reach])),Duration::EndOfTurn);
    swap(&mut g,a,b,true);assert!(g.state.blocks.iter().any(|p|p.blocker==y&&p.attacker==a));
}
#[test]
fn protection_blocks_the_exchange_without_partial_changes() {
    let mut g=game();let a=attack_as(&mut g,&PROTECTED);let b=attack(&mut g);
    let x=put(&mut g,&CREATURE,THEM);let y=put(&mut g,&RED,THEM);
    set_pairs(&mut g,&[(x,a),(y,b)]);let before=hash(&g);swap(&mut g,a,b,true);assert_eq!(hash(&g),before);
}
#[test]
fn attacker_exchanges_check_minimum_and_maximum_blocker_counts() {
    for def in [&MENACE,&LONE] {
        let mut g=game();let a=attack_as(&mut g,def);let b=attack(&mut g);
        let x=put(&mut g,&CREATURE,THEM);let y=put(&mut g,&CREATURE,THEM);let z=put(&mut g,&CREATURE,THEM);
        let pairs=if std::ptr::eq(def,&MENACE) {vec![(x,a),(y,a),(z,b)]}else{vec![(x,a),(y,b),(z,b)]};
        set_pairs(&mut g,&pairs);let before=g.state.blocks.clone();swap(&mut g,a,b,true);assert_eq!(g.state.blocks,before);
    }
}
#[test]
fn blocker_exchange_requires_capacity_for_the_entire_other_attack_group() {
    for other_multi in [false,true] {
        let mut g=game();let a=attack(&mut g);let b=attack(&mut g);let c=attack(&mut g);
        let x=put(&mut g,&MULTI,THEM);let y=put(&mut g,if other_multi {&MULTI}else{&CREATURE},THEM);
        set_pairs(&mut g,&[(x,a),(x,b),(y,c)]);let before=g.state.blocks.clone();swap(&mut g,x,y,false);
        if other_multi {assert!(g.state.blocks.iter().any(|p|p.blocker==y&&p.attacker==a));assert_eq!(edges(&g).len(),3);}
        else {assert_eq!(g.state.blocks,before);}
    }
}
#[test]
fn removing_and_reassigning_two_blockers_repeats_their_generic_triggers() {
    let mut g=game();let a=attack(&mut g);let b=attack(&mut g);
    let x=put(&mut g,&WITNESS,THEM);let y=put(&mut g,&WITNESS,THEM);
    set_pairs(&mut g,&[(x,a),(y,b)]);
    swap(&mut g,x,y,false);assert_eq!(edges(&g),[(y,a,true),(x,b,true)]);assert_eq!(g.state.triggers.len(),4);
}
#[test]
fn a_blocked_attacker_with_no_remaining_blocker_can_exchange_its_empty_cohort() {
    let mut g=game();let a=attack(&mut g);let b=attack(&mut g);let x=put(&mut g,&CREATURE,THEM);
    set_pairs(&mut g,&[(x,a)]);swap(&mut g,a,b,true);
    assert_eq!(g.state.blocks.len(),1);assert_eq!(g.state.blocks[0].attacker,b);assert!(g.is_blocked(a));assert!(g.is_blocked(b));
}
#[test]
fn a_surviving_blocker_can_exchange_its_empty_attack_group() {
    let mut g=game();let a=attack(&mut g);let b=attack(&mut g);
    let x=put(&mut g,&CREATURE,THEM);let y=put(&mut g,&CREATURE,THEM);
    set_pairs(&mut g,&[(x,a),(y,b)]);cx(&mut g).remove_from_combat(a);
    swap(&mut g,x,y,false);assert!(g.is_blocking(x));assert!(!g.is_blocking(y));assert_eq!(g.state.blocks[0].blocker,x);
}
#[test]
fn an_unrelated_existing_illegal_pair_does_not_prevent_the_exchange() {
    let mut g=game();let a=attack(&mut g);let b=attack(&mut g);let c=attack_as(&mut g,&FLYER);
    let x=put(&mut g,&CREATURE,THEM);let y=put(&mut g,&CREATURE,THEM);let z=put(&mut g,&CREATURE,THEM);
    set_pairs(&mut g,&[(x,a),(y,b),(z,c)]);swap(&mut g,a,b,true);
    assert!(g.state.blocks.iter().any(|p|p.blocker==x&&p.attacker==b));assert!(g.state.blocks.iter().any(|p|p.blocker==z&&p.attacker==c));
}
#[test]
fn declaration_requirements_are_not_imposed_again_by_an_exchange() {
    let mut g=game();let a=attack(&mut g);let b=attack(&mut g);
    let x=put(&mut g,&CREATURE,THEM);let y=put(&mut g,&CREATURE,THEM);let unused=put(&mut g,&CREATURE,THEM);
    set_pairs(&mut g,&[(x,a),(y,b)]);
    cx(&mut g).add_effect(unused,unused,EffectKind::MustBlockAttacker(a),Duration::EndOfTurn);
    swap(&mut g,a,b,true);assert!(g.state.blocks.iter().any(|p|p.blocker==x&&p.attacker==b));
}
#[test]
fn new_pairs_obey_whole_blocker_group_restrictions() {
    let mut g=game();let a=attack(&mut g);let x=put(&mut g,&CREATURE,THEM);let y=put(&mut g,&CREATURE,THEM);let z=put(&mut g,&CREATURE,THEM);
    put(&mut g,&CAP,THEM);
    let mark=g.state.mark();let log=g.log.len();assert!(!cx(&mut g).block_reassignment_allowed(&[],&[(x,a),(y,a),(z,a)]));
    assert_eq!(g.state.mark(),mark);assert_eq!(g.log.len(),log);
    assert!(cx(&mut g).block_reassignment_allowed(&[],&[(x,a),(y,a)]));
}
#[test]
fn the_solver_uses_a_legal_band_representative_without_extra_block_capacity() {
    let mut g=game();let a=attack_as(&mut g,&FLYER);let b=attack(&mut g);let x=put(&mut g,&CREATURE,THEM);
    update_attackers(&mut g,|a|for a in a {a.band=1;});
    assert!(cx(&mut g).block_reassignment_allowed(&[],&[(x,a)]));
    cx(&mut g).change_blocks(&[],&[],&[(x,a)]);assert_eq!(g.state.blocks.len(),2);
}
#[test]
fn distinct_bands_consume_distinct_block_allowances() {
    let mut g=game();let a=attack(&mut g);let b=attack(&mut g);let c=attack(&mut g);let d=attack(&mut g);
    update_attackers(&mut g,|a|{a[0].band=1;a[1].band=1;a[2].band=2;a[3].band=2;});
    let x=put(&mut g,&CREATURE,THEM);assert!(!cx(&mut g).block_reassignment_allowed(&[],&[(x,a),(x,c)]));
    let y=put(&mut g,&MULTI,THEM);assert!(cx(&mut g).block_reassignment_allowed(&[],&[(y,a),(y,b),(y,c),(y,d)]));
}
#[test]
fn a_shared_blocker_with_a_new_illegal_quality_prevents_the_exchange() {
    let mut g=game();let a=attack_as(&mut g,&FLYER);let b=attack(&mut g);let x=put(&mut g,&MULTI,THEM);
    set_pairs(&mut g,&[(x,a),(x,b)]);
    assert!(!cx(&mut g).block_reassignment_allowed(&[(x,a),(x,b)],&[(x,a),(x,b)]));
    let before=hash(&g);swap(&mut g,a,b,true);assert_eq!(hash(&g),before);assert!(edges(&g).is_empty());
}

#[test]
fn a_tapped_blocker_cannot_make_a_proposed_exchange_but_existing_blocks_persist() {
    for attacking in [false, true] {
        let mut g=game();let a=attack(&mut g);let b=attack(&mut g);let c=attack(&mut g);
        let x=put(&mut g,&CREATURE,THEM);let y=put(&mut g,&CREATURE,THEM);let z=put(&mut g,&CREATURE,THEM);
        set_pairs(&mut g,&[(x,a),(y,b),(z,c)]);
        g.state.update_obj(x,|o|o.tapped=true);g.state.update_obj(z,|o|o.tapped=true);
        let before=hash(&g);
        if attacking {swap(&mut g,a,b,true);} else {swap(&mut g,x,y,false);}
        assert_eq!(hash(&g),before,"a tapped creature cannot make its proposed block");
        g.state.update_obj(x,|o|o.tapped=false);
        if attacking {swap(&mut g,a,b,true);} else {swap(&mut g,x,y,false);}
        assert!(g.state.blocks.iter().any(|p|p.blocker==x&&p.attacker==b));
        assert!(g.state.blocks.iter().any(|p|p.blocker==z&&p.attacker==c),"unrelated tapped blocker keeps its block");
    }
}

#[test]
fn a_tapped_blocker_can_receive_an_empty_assignment() {
    let mut g=game();let a=attack(&mut g);let b=attack(&mut g);
    let x=put(&mut g,&CREATURE,THEM);let y=put(&mut g,&CREATURE,THEM);
    set_pairs(&mut g,&[(x,a),(y,b)]);cx(&mut g).remove_from_combat(a);
    g.state.update_obj(y,|o|o.tapped=true);
    swap(&mut g,x,y,false);
    assert_eq!(g.state.blocks.len(),1);
    assert_eq!((g.state.blocks[0].blocker,g.state.blocks[0].attacker),(x,b));
    assert!(!g.is_blocking(y));
}
