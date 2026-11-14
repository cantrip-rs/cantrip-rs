//! Random costs are exact, last, and selected through the journalled random-choice authority (CR 601.2h, 701.9b).
use super::*;
use crate::Game as RulesGame;
use cantrip_state::{Frame,FrameKind};
const YOU:PlayerId=PlayerId(0);
static SOURCE:CardDef=CardDef {name:"Random discard cost probe",types:TypeSet::of(&[CardType::Artifact]),..CardDef::BLANK};
static CARD:CardDef=CardDef {name:"Random discarded card probe",types:TypeSet::of(&[CardType::Creature]),power:Some(1),toughness:Some(1),..CardDef::BLANK};
static SPECIAL:CardDef=CardDef {name:"Random special-action probe",abilities:&[ability(Cost::sequence(&[
    Cost {life:2,..Cost::NONE},Cost {discard_at_random:1,..Cost::NONE}]),|_,_|Ok(()),"Pay 2 life, discard a card at random: Do nothing.").special_action()],..SOURCE};
fn board(n:usize)->(RulesGame,ObjectId,Vec<ObjectId>) {
    let cards=Box::leak(Box::new(CardTable::new([&SOURCE,&CARD,&SPECIAL])));
    let mut g=RulesGame::new(cards,&crate::Setup {decks:vec![vec![],vec![]],..Default::default()});
    while !g.state.agenda.is_empty(){g.state.frame_remove(0);}
    g.state.frame_push(Frame::new(FrameKind::Priority));
    let source=g.state.push_obj(Obj::new(cards.id_of(&SOURCE),YOU,Zone::BATTLEFIELD));g.state.zone_push(Zone::BATTLEFIELD,source);
    let hand=(0..n).map(|_|{let id=g.state.push_obj(Obj::new(cards.id_of(&CARD),YOU,Zone::hand(YOU)));g.state.zone_push(Zone::hand(YOU),id);id}).collect();(g,source,hand)
}
fn cx(g:&mut RulesGame)->Cx<'_>{Cx {g,frame:0,cursor:0,mana_source:None,entering:None,lineage:Vec::new(),damage:None,asked_at:None,doer:Some(YOU),producing:None}}
#[test]
fn random_discard_is_after_life_and_gives_no_card_choice() {
    let (mut g,source,hand)=board(3);let cost=Cost {life:2,discard_at_random:1,..Cost::NONE};
    assert!(can_pay_other(&cx(&mut g),source,YOU,&cost));
    pay_resolution_cost(&mut cx(&mut g),YOU,source,&cost,ManaCost::ZERO,0,0).unwrap();
    assert_eq!(g.state.player(YOU).life,18);assert_eq!(g.state.zone(Zone::hand(YOU)).len(),2);
    let life=g.log.iter().position(|r|matches!(r.event,Event::LifeChange {..})).unwrap();
    let random=g.log.iter().position(|r|matches!(r.event,Event::ChosenAtRandom {..})).unwrap();assert!(life<random);
    assert!(g.pending().is_none(),"the payer never chooses the random card");
    assert_eq!(hand.iter().filter(|&&id|g.state.obj(id).became.is_some()).count(),1);
}
#[test]
fn random_discard_is_exact_without_replacement_and_rolls_back_its_random_choice() {
    let (mut g,source,_)=board(3);let cost=Cost {discard_at_random:1,..Cost::NONE}.times(2);
    let mark=g.state.mark();let log=g.log.len();
    let first=choose_discards_apart(&mut cx(&mut g),YOU,source,&cost,None,&[]).unwrap();
    assert_eq!(first.cards.len(),2);assert_ne!(first.cards[0],first.cards[1]);
    g.state.rollback(mark);g.log.truncate(log);
    let again=choose_discards_apart(&mut cx(&mut g),YOU,source,&cost,None,&[]).unwrap();assert_eq!(first.cards,again.cards);
    let (mut g,source,hand)=board(2);let chosen=choose_discards_apart(&mut cx(&mut g),YOU,source,&cost,None,&[]).unwrap();
    assert_eq!(chosen.cards,hand);assert!(!g.log.iter().any(|r|matches!(r.event,Event::ChosenAtRandom {..})),"taking the whole hand needs no random draw");
    let impossible=Cost {discard_at_random:3,..Cost::NONE};assert!(!can_pay_other(&cx(&mut g),source,YOU,&impossible));
    assert!(choose_discards_apart(&mut cx(&mut g),YOU,source,&impossible,None,&[]).is_err());
}
#[test]
fn random_discard_cannot_feed_an_earlier_nonrandom_graveyard_cost() {
    let (mut g,source,_)=board(2);
    let cost=Cost {discard_at_random:1,exile_from_graveyard:1,..Cost::NONE};
    assert!(!can_pay_other(&cx(&mut g),source,YOU,&cost),"random payment must be last, so its card cannot fund this exile");
    let ordinary=Cost {discard:Discards::one(1,None),discard_at_random:0,..cost};
    assert!(can_pay_other(&cx(&mut g),source,YOU,&ordinary),"a chosen discard may be paid before the exile");
}

#[test]
fn a_special_actions_separate_random_component_is_also_paid_after_life() {
    let (mut g,_,_)=board(3);
    let card=g.cards.id_of(&SPECIAL);let source=g.state.push_obj(Obj::new(card,YOU,Zone::BATTLEFIELD));g.state.zone_push(Zone::BATTLEFIELD,source);
    special_action(&mut cx(&mut g),YOU,source,AbilityRef {card,index:0},None).unwrap();
    assert_eq!(g.state.player(YOU).life,18);assert_eq!(g.state.zone(Zone::hand(YOU)).len(),2);
    let life=g.log.iter().position(|r|matches!(r.event,Event::LifeChange {..})).unwrap();
    let random=g.log.iter().position(|r|matches!(r.event,Event::ChosenAtRandom {..})).unwrap();assert!(life<random);
    assert!(g.state.stack.is_empty());
}
