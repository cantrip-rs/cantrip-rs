//! Typed keyword identity, independent of display labels and preserved through the normal layers (CR 702.23/24, 613).
use crate::{Cx,Game,Setup};
use cantrip_api::*;
use cantrip_api::Game as Rules;
const YOU:PlayerId=PlayerId(0);
static UPKEEP:CardDef=CardDef {name:"Typed upkeep probe",types:TypeSet::of(&[CardType::Enchantment]),abilities:&[
    cantrip_mechanics::cumulative_upkeep!(cantrip_mechanics::cumulative_upkeep::Payment::Mana(mana("{1}")),"Different label")],..CardDef::BLANK};
static RAMPAGE:CardDef=CardDef {name:"Typed rampage probe",abilities:&[cantrip_mechanics::rampage::trigger::<2>("Different label")],..UPKEEP};
static DECOY:CardDef=CardDef {name:"Keyword label decoy",abilities:&[triggered(when(EventMask::STEP_BEGAN,|_,_,_|None),|_,_|Ok(()),"Cumulative upkeep {1}; rampage 2")],..UPKEEP};
static GRANT:CardDef=CardDef {name:"Typed keyword grant probe",abilities:&[],delayed:&[cantrip_mechanics::rampage::trigger::<3>("Rampage 3")],..UPKEEP};
fn board()->(Game,ObjectId,ObjectId,ObjectId,ObjectId) {
    let cards=Box::leak(Box::new(CardTable::new([&UPKEEP,&RAMPAGE,&DECOY,&GRANT])));
    let mut g=Game::new(cards,&Setup {decks:vec![vec![],vec![]],..Default::default()});
    let mut put=|d|{let id=g.state.push_obj(Obj::new(cards.id_of(d),YOU,Zone::BATTLEFIELD));g.state.zone_push(Zone::BATTLEFIELD,id);id};
    let u=put(&UPKEEP);let r=put(&RAMPAGE);let d=put(&DECOY);let a=put(&GRANT);g.state.see_statics();(g,u,r,d,a)
}
fn cx(g:&mut Game)->Cx<'_>{Cx {g,frame:usize::MAX,cursor:0,mana_source:None,entering:None,lineage:Vec::new(),damage:None,asked_at:None,doer:None,producing:None}}
#[test]
fn keyword_identity_is_typed_and_not_inferred_from_text() {
    let (g,u,r,d,_)=board();
    assert!(g.has_keyword_family(u,KeywordFamily::CumulativeUpkeep));assert!(!g.has_keyword_family(u,KeywordFamily::Rampage));
    assert!(g.has_keyword_family(r,KeywordFamily::Rampage));assert!(!g.has_keyword_family(r,KeywordFamily::CumulativeUpkeep));
    assert!(!g.has_keyword_family(d,KeywordFamily::CumulativeUpkeep));assert!(!g.has_keyword_family(d,KeywordFamily::Rampage));
}
#[test]
fn acquired_copied_and_lost_keyword_instances_use_current_abilities() {
    let (mut g,u,_,_,giver)=board();let ramp=AbilityRef::delayed(g.cards.id_of(&GRANT),0);
    cx(&mut g).add_effect(giver,u,EffectKind::AddAbility(ramp),Duration::EndOfTurn);
    assert!(g.has_keyword_family(u,KeywordFamily::CumulativeUpkeep));assert!(g.has_keyword_family(u,KeywordFamily::Rampage));
    cx(&mut g).add_effect(giver,u,EffectKind::LoseAllAbilities,Duration::EndOfTurn);
    assert!(!g.has_keyword_family(u,KeywordFamily::CumulativeUpkeep));assert!(!g.has_keyword_family(u,KeywordFamily::Rampage));
    cx(&mut g).add_effect(giver,u,EffectKind::AddAbility(ramp),Duration::EndOfTurn);assert!(g.has_keyword_family(u,KeywordFamily::Rampage));
    let (mut g,u,_,_,_)=board();let card=g.cards.id_of(&RAMPAGE);cx(&mut g).set_copy(u,Copiable::of(card));
    assert!(g.has_keyword_family(u,KeywordFamily::Rampage));assert!(!g.has_keyword_family(u,KeywordFamily::CumulativeUpkeep));
}
