//! Explicit unspent-mana loss (Mana Short, Power Sink, Drain Power), separate from CR 500.5 step/phase emptying.
use crate::{Cx,Game,Setup};
use cantrip_state::{Frame,FrameKind};
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
    g.state.empty_mana_keeping(YOU,ManaLasts::Step,[true;6],None);assert_eq!(g.state.player(YOU).pool.get(Mana::U),3);
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
    g.state.empty_mana_keeping(YOU,ManaLasts::Step,kept[0].1,None);
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
fn becomes_red(_: &dyn Query,asker:&Asker,player:PlayerId)->Option<Mana> {(player==asker.controller).then_some(Mana::R)}
static OZAI:CardDef=CardDef {name:"Mana-becomes probe",types:TypeSet::of(&[CardType::Enchantment]),mana_lost_becomes:&[becomes_red],..CardDef::BLANK};
fn ozai_board()->(Game,ObjectId) {
    let cards=Box::leak(Box::new(CardTable::new([&OZAI])));
    let mut g=Game::new(cards,&Setup {decks:vec![vec![],vec![]],..Default::default()});
    let id=g.state.push_obj(Obj::new(cards.id_of(&OZAI),YOU,Zone::BATTLEFIELD));g.state.zone_push(Zone::BATTLEFIELD,id);g.state.see_statics();(g,id)
}
#[test]
fn mana_that_would_be_lost_becomes_red_for_the_controller_only() {
    // Ozai, the Phoenix King (CR 106.4, 614.1a): "If you would lose unspent mana, that mana becomes red instead."
    let (mut g,source)=ozai_board();
    let becomes=crate::static_rules::mana_lost_becomes(&cx(&mut g));
    assert_eq!(becomes,vec![(YOU,Mana::R,source)],"only the controller's mana, and no one else's");
    g.state.add_mana(YOU,Mana::U,3,source,None,ManaLasts::Step,false);
    g.state.add_mana(YOU,Mana::R,1,source,None,ManaLasts::Step,false);
    g.state.add_mana(YOU,Mana::G,2,source,None,ManaLasts::Turn,false);
    g.state.add_mana(YOU,Mana::B,4,source,Some(ManaRestriction::Spells(TypeSet::of(&[CardType::Creature]))),ManaLasts::Step,false);
    g.state.empty_mana_keeping(YOU,ManaLasts::Step,[false;6],Some(Mana::R));
    let pool=g.state.player(YOU).pool;
    assert_eq!((pool.get(Mana::R),pool.get(Mana::U),pool.get(Mana::G),pool.get(Mana::B)),(4,0,2,0),"the {{U}}{{U}}{{U}} is red now; the {{G}}{{G}} that lasts until end of turn was not lost");
    let restricted=g.state.restricted_of(YOU).map(|(r,p)|(r,p.get(Mana::R),p.get(Mana::B))).collect::<Vec<_>>();
    assert_eq!(restricted.len(),1);assert_eq!((restricted[0].1,restricted[0].2),(4,0),"restricted mana turns red and stays restricted");
    let from=g.state.mana_from_of(YOU).map(|e|e.pool.total()).sum::<u32>();assert_eq!(from,pool.total()+4,"the sources still add up to the pool and the restricted mana");
    // A second step ends: the red mana stays again.
    g.state.empty_mana_keeping(YOU,ManaLasts::Step,[false;6],Some(Mana::R));
    assert_eq!(g.state.player(YOU).pool.get(Mana::R),4);
}
#[test]
fn an_explicit_loss_of_unspent_mana_turns_it_red_instead_and_loses_nothing() {
    let (mut g,source)=ozai_board();
    g.state.add_mana(THEM,Mana::U,2,source,None,ManaLasts::Step,false);
    g.state.add_mana(YOU,Mana::U,3,source,None,ManaLasts::Step,false);
    g.state.add_mana(YOU,Mana::G,2,source,None,ManaLasts::Turn,false);
    assert!(cx(&mut g).empty_mana_pool(YOU).is_empty(),"nothing was lost");
    assert_eq!(g.state.player(YOU).pool.get(Mana::R),5);assert_eq!(g.state.player(YOU).pool.get(Mana::U),0);
    assert_eq!(cx(&mut g).empty_mana_pool(THEM).get(Mana::U),2,"another player's mana is lost as usual");
}
fn becomes_colorless(_: &dyn Query,asker:&Asker,player:PlayerId)->Option<Mana> {(player==asker.controller).then_some(Mana::C)}
static STONE:CardDef=CardDef {name:"Mana-becomes-colorless probe",types:TypeSet::of(&[CardType::Artifact]),mana_lost_becomes:&[becomes_colorless],..CardDef::BLANK};
/// A game in its first main phase, between steps, with the probes `defs` on YOU's battlefield and {U}{U}{U} in YOU's pool and
/// {U} in THEM's.
fn two_becomes(defs:&[&'static CardDef])->(Game,Vec<ObjectId>) {
    let cards=Box::leak(Box::new(CardTable::new([&OZAI,&STONE])));
    let mut g=Game::new(cards,&Setup {decks:vec![vec![],vec![]],..Default::default()});
    while !g.state.agenda.is_empty(){g.state.frame_remove(0);}
    g.state.update_turn(|t|{t.number=1;t.active=YOU;t.priority=YOU;t.set_step(Step::PrecombatMain);});
    let ids=defs.iter().map(|d|{let id=g.state.push_obj(Obj::new(cards.id_of(d),YOU,Zone::BATTLEFIELD));g.state.zone_push(Zone::BATTLEFIELD,id);id}).collect::<Vec<_>>();
    g.state.see_statics();
    g.state.add_mana(YOU,Mana::U,3,ids[0],None,ManaLasts::Step,false);g.state.add_mana(THEM,Mana::U,1,ids[0],None,ManaLasts::Step,false);
    g.verify=true;
    (g,ids)
}
/// The main phase ends: the question of which effect applies, asked of YOU, and its sources.
fn ends_asking(g:&mut Game)->(u32,Vec<ObjectId>,Vec<String>) {
    crate::turn::next_step(g);g.run();
    let r=g.pending().expect("a question as the step ends").clone();
    assert_eq!(r.player,YOU);assert_eq!(g.state.turn.step(),Step::PrecombatMain,"asked as the step ends, before the next begins");
    let RequestKind::ChooseReplacement {sources,hints,mana:true,..}=r.kind else {panic!("which effect applies: {:?}",r.kind)};
    (r.id,sources,hints.iter().map(|h|h.to_string()).collect())
}
#[test]
fn with_two_types_the_player_chooses_which_one_their_lost_mana_becomes() {
    // Ozai, the Phoenix King beside Horizon Stone (CR 616.1): the affected player chooses which replacement effect applies;
    // once one has, the mana is not lost and the other has nothing to replace (CR 616.1f).
    let (mut g,ids)=two_becomes(&[&OZAI,&STONE]);
    let (id,sources,hints)=ends_asking(&mut g);
    assert_eq!(sources,ids,"each effect offered once, by its source");
    assert_eq!(hints,vec!["That mana becomes red instead","That mana becomes colorless instead"]);
    let asked=g.at;
    // Each answer from the same position: going back to the question undoes the emptying exactly.
    for (pick,mana) in [(0u8,Mana::R),(1,Mana::C)] {
        g.goto(asked);
        assert_eq!(g.pending().map(|r|r.id),Some(id));assert_eq!(g.state.player(YOU).pool.get(Mana::U),3);
        g.answer(id,YOU,Answer::Index(pick)).unwrap();
        assert_eq!(g.state.turn.step(),Step::BeginCombat,"the game goes on to the next step");
        let pool=g.state.player(YOU).pool;
        assert_eq!((pool.get(mana),pool.get(Mana::U),pool.total()),(3,0,3),"the {{U}}{{U}}{{U}} became {mana:?} only");
        assert!(g.state.player(THEM).pool.is_empty(),"another player's mana is lost as usual");
    }
}
#[test]
fn one_type_or_nothing_to_lose_asks_nothing() {
    // Two effects that turn it red do the same: nothing to choose.
    let (mut g,_)=two_becomes(&[&OZAI,&OZAI]);
    crate::turn::next_step(&mut g);g.run();
    assert!(!matches!(g.pending().map(|r|&r.kind),Some(RequestKind::ChooseReplacement {..})));
    assert_eq!(g.state.turn.step(),Step::BeginCombat);assert_eq!(g.state.player(YOU).pool.get(Mana::R),3);
    // With both types but no mana to lose, there is no event to replace.
    let (mut g,ids)=two_becomes(&[&OZAI,&STONE]);
    g.state.empty_mana(YOU,ManaLasts::Turn);g.state.add_mana(YOU,Mana::G,2,ids[0],None,ManaLasts::Turn,false);
    crate::turn::next_step(&mut g);g.run();
    assert!(!matches!(g.pending().map(|r|&r.kind),Some(RequestKind::ChooseReplacement {..})),"mana that lasts the turn is not lost");
    assert_eq!(g.state.turn.step(),Step::BeginCombat);assert_eq!(g.state.player(YOU).pool.get(Mana::G),2);
}
#[test]
fn kept_mana_is_not_lost_and_is_never_turned() {
    // Mana a static ability keeps is not lost (CR 500.5), so neither effect would replace anything: no question.
    let cards=Box::leak(Box::new(CardTable::new([&HOLDER,&OZAI,&STONE])));
    let mut g=Game::new(cards,&Setup {decks:vec![vec![],vec![]],..Default::default()});
    while !g.state.agenda.is_empty(){g.state.frame_remove(0);}
    g.state.update_turn(|t|{t.number=1;t.active=YOU;t.priority=YOU;t.set_step(Step::PrecombatMain);});
    for d in [&HOLDER,&OZAI,&STONE] {let id=g.state.push_obj(Obj::new(cards.id_of(d),YOU,Zone::BATTLEFIELD));g.state.zone_push(Zone::BATTLEFIELD,id);}
    g.state.see_statics();let source=g.state.battlefield()[0];
    g.state.add_mana(YOU,Mana::U,3,source,None,ManaLasts::Step,false);
    crate::turn::next_step(&mut g);g.run();
    assert!(!matches!(g.pending().map(|r|&r.kind),Some(RequestKind::ChooseReplacement {..})));
    assert_eq!(g.state.player(YOU).pool.get(Mana::U),3);
}
#[test]
fn an_explicit_loss_with_two_types_asks_and_is_replayed_with_the_answer() {
    let (mut g,_)=two_becomes(&[&OZAI,&STONE]);
    g.state.frame_push(Frame::new(FrameKind::Settle));
    let mark=g.state.mark();
    assert!(cx_at(&mut g).empty_mana_pool(YOU).is_empty(),"nothing is lost either way");
    let r=g.pending().expect("which effect applies").clone();
    assert!(matches!(r.kind,RequestKind::ChooseReplacement {mana:true,..}));assert_eq!(r.player,YOU);
    // Replayed with the answer, as the run loop does: colorless.
    g.state.revert(mark);g.state.update_frame(0,|f|{f.pending=None;f.answers=vec![(Some(YOU),Answer::Index(1))];});
    assert!(cx_at(&mut g).empty_mana_pool(YOU).is_empty());
    assert!(g.pending().is_none());let pool=g.state.player(YOU).pool;assert_eq!((pool.get(Mana::C),pool.total()),(3,3));
}
fn cx_at(g:&mut Game)->Cx<'_>{Cx {g,frame:0,cursor:0,mana_source:None,entering:None,lineage:Vec::new(),damage:None,asked_at:None,doer:None,producing:None, reached:None}}