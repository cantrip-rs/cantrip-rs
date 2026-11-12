//! Typed keyword identity, independent of display labels and preserved through the normal layers (CR 702.24, 613).
use crate::{Cx,Game,Setup};
use cantrip_api::*;
use cantrip_api::Game as Rules;
const YOU:PlayerId=PlayerId(0);
static UPKEEP:CardDef=CardDef {name:"Typed upkeep probe",types:TypeSet::of(&[CardType::Enchantment]),abilities:&[
    cantrip_mechanics::cumulative_upkeep!(cantrip_mechanics::cumulative_upkeep::Payment::Mana(mana("{1}")),"Different label")],..CardDef::BLANK};
static PLAIN:CardDef=CardDef {name:"Typed plain probe",abilities:&[],..UPKEEP};
static DECOY:CardDef=CardDef {name:"Keyword label decoy",abilities:&[triggered(when(EventMask::STEP_BEGAN,|_,_,_|None),|_,_|Ok(()),"Cumulative upkeep {1}")],..UPKEEP};
static GRANT:CardDef=CardDef {name:"Typed keyword grant probe",abilities:&[],delayed:&[
    cantrip_mechanics::cumulative_upkeep!(cantrip_mechanics::cumulative_upkeep::Payment::Mana(mana("{2}")),"Cumulative upkeep {2}")],..UPKEEP};
fn board()->(Game,ObjectId,ObjectId,ObjectId,ObjectId) {
    let cards=Box::leak(Box::new(CardTable::new([&UPKEEP,&PLAIN,&DECOY,&GRANT])));
    let mut g=Game::new(cards,&Setup {decks:vec![vec![],vec![]],..Default::default()});
    let mut put=|d|{let id=g.state.push_obj(Obj::new(cards.id_of(d),YOU,Zone::BATTLEFIELD));g.state.zone_push(Zone::BATTLEFIELD,id);id};
    let u=put(&UPKEEP);let p=put(&PLAIN);let d=put(&DECOY);let a=put(&GRANT);g.state.see_statics();(g,u,p,d,a)
}
fn cx(g:&mut Game)->Cx<'_>{Cx {g,frame:usize::MAX,cursor:0,mana_source:None,entering:None,lineage:Vec::new(),damage:None,asked_at:None,doer:None,producing:None,reached:None}}
#[test]
fn keyword_identity_is_typed_and_not_inferred_from_text() {
    let (g,u,p,d,_)=board();
    assert!(g.has_keyword_family(u,KeywordFamily::CumulativeUpkeep));assert!(!g.has_keyword_family(p,KeywordFamily::CumulativeUpkeep));
    assert!(!g.has_keyword_family(d,KeywordFamily::CumulativeUpkeep));
}
#[test]
fn acquired_copied_and_lost_keyword_instances_use_current_abilities() {
    let (mut g,_,p,_,giver)=board();let upkeep=AbilityRef::delayed(g.cards.id_of(&GRANT),0);
    cx(&mut g).add_effect(giver,p,EffectKind::AddAbility(upkeep),Duration::EndOfTurn);
    assert!(g.has_keyword_family(p,KeywordFamily::CumulativeUpkeep));
    cx(&mut g).add_effect(giver,p,EffectKind::LoseAllAbilities,Duration::EndOfTurn);
    assert!(!g.has_keyword_family(p,KeywordFamily::CumulativeUpkeep));
    cx(&mut g).add_effect(giver,p,EffectKind::AddAbility(upkeep),Duration::EndOfTurn);assert!(g.has_keyword_family(p,KeywordFamily::CumulativeUpkeep));
    let (mut g,u,p,_,_)=board();let card=g.cards.id_of(&UPKEEP);cx(&mut g).set_copy(p,Copiable::of(card));
    assert!(g.has_keyword_family(p,KeywordFamily::CumulativeUpkeep));
    let card=g.cards.id_of(&PLAIN);cx(&mut g).set_copy(u,Copiable::of(card));assert!(!g.has_keyword_family(u,KeywordFamily::CumulativeUpkeep));
}
