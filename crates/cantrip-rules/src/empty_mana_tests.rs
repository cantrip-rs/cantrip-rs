//! Explicit unspent-mana loss (Mana Short, Power Sink, Drain Power), separate from CR 500.5 step/phase emptying.
use crate::{Cx,Game,Setup};
use cantrip_api::*;
use cantrip_api::Game as Rules;
const YOU:PlayerId=PlayerId(0);
const THEM:PlayerId=PlayerId(1);
fn keep(_: &dyn Query,_:&Asker,_:PlayerId,_:Mana)->bool {true}
static HOLDER:CardDef=CardDef {name:"Explicit mana-loss probe",types:TypeSet::of(&[CardType::Enchantment]),mana_kept:&[keep],..CardDef::BLANK};
fn board()->(Game,ObjectId) {
    let cards=Box::leak(Box::new(CardTable::new([&HOLDER])));
    let mut g=Game::new(cards,&Setup {decks:vec![vec![],vec![]],..Default::default()});
    let id=g.state.push_obj(Obj::new(cards.id_of(&HOLDER),YOU,Zone::BATTLEFIELD));g.state.zone_push(Zone::BATTLEFIELD,id);g.state.see_statics();(g,id)
}
fn cx(g:&mut Game)->Cx<'_>{Cx {g,frame:usize::MAX,cursor:0,mana_source:None,entering:None,lineage:Vec::new(),damage:None,asked_at:None,doer:None,producing:None, reached:None}}
#[test]
fn explicit_loss_empties_every_pool_and_lifetime_even_under_a_step_boundary_keep_rule() {
    let (mut g,source)=board();
    g.state.add_mana(YOU,Mana::U,3,source,None,ManaLasts::Step,false);
    g.state.add_mana(YOU,Mana::R,2,source,None,ManaLasts::Combat,true);
    g.state.add_mana(YOU,Mana::G,4,source,None,ManaLasts::Turn,false);
    g.state.add_mana(YOU,Mana::U,5,source,Some(ManaRestriction::Spells(TypeSet::of(&[CardType::Creature]))),ManaLasts::Step,true);
    g.state.add_mana(THEM,Mana::B,7,source,None,ManaLasts::Turn,false);
    let kept=crate::static_rules::mana_kept(&cx(&mut g));assert_eq!(kept.iter().find(|(p,_)|*p==YOU).unwrap().1,[true;6]);
    // Show the distinction: the same board's boundary keep rule preserves its blue step mana.
    g.state.empty_mana_keeping(YOU,ManaLasts::Step,[true;6]);assert_eq!(g.state.player(YOU).pool.get(Mana::U),3);
    let lost=cx(&mut g).empty_mana_pool(YOU);assert_eq!(lost.get(Mana::U),8);assert_eq!(lost.get(Mana::R),2);assert_eq!(lost.get(Mana::G),4);
    assert!(g.state.player(YOU).pool.is_empty());assert!(g.state.player(YOU).lasting_mana().is_empty());
    assert!(!g.state.has_restricted_mana(YOU));assert_eq!(g.state.mana_from_of(YOU).count(),0);
    assert_eq!(g.state.player(THEM).pool.get(Mana::B),7);assert_eq!(g.state.mana_from_of(THEM).count(),1);
    assert!(cx(&mut g).empty_mana_pool(YOU).is_empty(),"a second loss has nothing to lose");
}
#[test]
fn rollback_restores_pool_restrictions_snow_and_lifetimes_then_later_mana_still_works() {
    let (mut g,source)=board();
    g.state.add_mana(YOU,Mana::G,2,source,None,ManaLasts::Turn,true);
    g.state.add_mana(YOU,Mana::C,3,source,Some(ManaRestriction::Spells(TypeSet::of(&[CardType::Creature]))),ManaLasts::Step,false);
    let mark=g.state.mark();let before=g.state.mana_from_of(YOU).copied().collect::<Vec<_>>();
    cx(&mut g).empty_mana_pool(YOU);g.state.rollback(mark);
    assert_eq!(g.state.player(YOU).pool.get(Mana::G),2);assert_eq!(g.state.player(YOU).lasting_mana().get(Mana::G),2);
    assert_eq!(g.state.restricted_of(YOU).next().unwrap().1.get(Mana::C),3);
    assert_eq!(g.state.mana_from_of(YOU).copied().collect::<Vec<_>>(),before);
    cx(&mut g).empty_mana_pool(YOU);cx(&mut g).add_mana(YOU,Mana::R,1);
    assert_eq!(g.state.player(YOU).pool.get(Mana::R),1);assert_eq!(g.state.mana_from_of(YOU).count(),1);
}
static BARE:CardDef=CardDef {name:"Resolved-effect probe",types:TypeSet::of(&[CardType::Enchantment]),..CardDef::BLANK};
#[test]
fn a_resolved_keep_effect_keeps_one_type_for_its_controller_until_it_ends() {
    // The Last Agni Kai (CR 106.4, 500.5): "until end of turn, you don't lose unspent red mana as steps and phases end".
    let cards=Box::leak(Box::new(CardTable::new([&BARE])));
    let mut g=Game::new(cards,&Setup {decks:vec![vec![],vec![]],..Default::default()});
    let source=g.state.push_obj(Obj::new(cards.id_of(&BARE),YOU,Zone::BATTLEFIELD));g.state.zone_push(Zone::BATTLEFIELD,source);g.state.see_statics();
    assert!(crate::static_rules::mana_kept(&cx(&mut g)).is_empty(),"nothing keeps mana before the effect resolves");
    cx(&mut g).add_player_effect(YOU,source,EffectKind::ManaKept(Mana::R),Duration::EndOfTurn);
    let kept=crate::static_rules::mana_kept(&cx(&mut g));
    assert_eq!(kept.len(),1);assert_eq!(kept[0].0,YOU);assert_eq!(kept[0].1,[false,false,false,true,false,false],"only the controller's red");
    g.state.add_mana(YOU,Mana::R,2,source,None,ManaLasts::Step,false);g.state.add_mana(YOU,Mana::U,3,source,None,ManaLasts::Step,false);
    g.state.empty_mana_keeping(YOU,ManaLasts::Step,kept[0].1);
    assert_eq!(g.state.player(YOU).pool.get(Mana::R),2);assert_eq!(g.state.player(YOU).pool.get(Mana::U),0);
}
#[test]
fn resolved_keep_effects_of_one_player_are_merged_and_those_of_another_are_kept_apart() {
    let cards=Box::leak(Box::new(CardTable::new([&BARE])));
    let mut g=Game::new(cards,&Setup {decks:vec![vec![],vec![]],..Default::default()});
    let source=g.state.push_obj(Obj::new(cards.id_of(&BARE),YOU,Zone::BATTLEFIELD));g.state.zone_push(Zone::BATTLEFIELD,source);g.state.see_statics();
    cx(&mut g).add_player_effect(YOU,source,EffectKind::ManaKept(Mana::R),Duration::EndOfTurn);
    cx(&mut g).add_player_effect(YOU,source,EffectKind::ManaKept(Mana::G),Duration::EndOfTurn);
    cx(&mut g).add_player_effect(THEM,source,EffectKind::ManaKept(Mana::U),Duration::EndOfTurn);
    let kept=crate::static_rules::mana_kept(&cx(&mut g));
    assert_eq!(kept.len(),2,"one entry a player");
    assert_eq!(kept.iter().find(|k|k.0==YOU).unwrap().1,[false,false,false,true,true,false],"red and green for the one who has both");
    assert_eq!(kept.iter().find(|k|k.0==THEM).unwrap().1,[false,true,false,false,false,false],"blue for the other");
}
