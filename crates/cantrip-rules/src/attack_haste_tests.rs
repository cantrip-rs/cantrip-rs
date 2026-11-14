//! Instill Energy/Chaos Lord: CR 302.6 and 609.4 permit attacks only.
use crate::{Cx, Game, Setup};
use cantrip_api::*;
use cantrip_api::Game as Rules;
use cantrip_state::{Frame, FrameKind};
const YOU:PlayerId=PlayerId(0);
const THEM:PlayerId=PlayerId(1);
const THIRD:PlayerId=PlayerId(2);
fn all(_: &dyn Query,_:&Asker,_:ObjectId,_:Target)->bool{true}
static SPELL:CardDef=CardDef {name:"Attack exception priority holder",types:TypeSet::of(&[CardType::Instant]),has_cost:true,spell:spell(&[],|_,_|Ok(())),..CardDef::BLANK};
static CREATURE:CardDef=CardDef {name:"Attack exception creature",types:TypeSet::of(&[CardType::Creature]),power:Some(2),toughness:Some(2),abilities:&[mana_ability(TAP,add(Mana::G),"{T}: Add {G}.")],..CardDef::BLANK};
static DEFENDER:CardDef=CardDef {name:"Attack exception defender",keywords:Keywords::of(&[Keyword::Defender]),..CREATURE};
static RULE:CardDef=CardDef {name:"Attack exception rule",types:TypeSet::of(&[CardType::Enchantment]),attacks_as_though_haste:&[all],..CardDef::BLANK};
static ONLY:CardDef=CardDef {name:"Attack exception one defender",attacks_as_though_haste:&[|_,_,_,at|at==Target::Player(THEM)],..RULE};
static BAN:CardDef=CardDef {name:"Attack exception independent ban",attack_rules:&[|_,_,_,_|false],attacks_as_though_haste:&[],..RULE};
static DECOY:CardDef=CardDef {name:"Attack exception plain enchantment",attacks_as_though_haste:&[],..RULE};
fn board()->Game {
    let cards=Box::leak(Box::new(CardTable::new([&CREATURE,&DEFENDER,&RULE,&ONLY,&BAN,&DECOY,&SPELL])));
    let filler=cards.id_of(&DECOY);
    let mut g=Game::new(cards,&Setup {decks:vec![vec![filler;20];3],shuffle:false,..Default::default()});
    while !g.state.agenda.is_empty(){g.state.frame_remove(0);}g.state.frame_push(Frame::new(FrameKind::BeginStep));
    g.state.update_turn(|t|{t.number=1;t.active=YOU;t.priority=YOU;t.set_step(Step::DeclareAttackers);});g.state.update_player(YOU,|p|p.turn_began=1);
    let spell=g.state.push_obj(Obj::new(cards.id_of(&SPELL),YOU,Zone::hand(YOU)));g.state.zone_push(Zone::hand(YOU),spell);g
}
fn put(g:&mut Game,d:&'static CardDef,p:PlayerId)->ObjectId{let id=g.state.push_obj(Obj::new(g.cards.id_of(d),p,Zone::BATTLEFIELD));g.state.zone_push(Zone::BATTLEFIELD,id);g.state.update_obj(id,|o|o.controlled_since=1);g.state.see_statics();id}
fn cx(g:&mut Game)->Cx<'_>{Cx {g,frame:0,cursor:0,mana_source:None,entering:None,lineage:Vec::new(),damage:None,asked_at:None,doer:None,producing:None}}
fn offered(g:&mut Game)->Vec<ObjectId>{g.run();match &g.pending().unwrap().kind {RequestKind::DeclareAttackers {eligible,..}=>eligible.clone(),RequestKind::Priority {..}=>vec![],other=>panic!("{other:?}")}}
#[test]
fn the_attack_exception_does_not_grant_haste_or_unlock_tap_symbol_abilities() {
    let mut g=board();put(&mut g,&RULE,YOU);let creature=put(&mut g,&CREATURE,YOU);
    assert!(!g.chars(creature).has(Keyword::Haste));
    assert!(!crate::actions::priority_options(&mut cx(&mut g),YOU).iter().any(|o|o.source==Some(creature)&&o.kind==ActionKind::ActivateAbility));
    assert_eq!(offered(&mut g),vec![creature]);
}
#[test]
fn defender_tapped_wrong_controller_and_independent_bans_still_prevent_attacking() {
    for case in 0..4 {
        let mut g=board();put(&mut g,&RULE,YOU);let creature=put(&mut g,if case==0 {&DEFENDER}else{&CREATURE},if case==2 {THEM}else{YOU});
        if case==1 {g.state.update_obj(creature,|o|o.tapped=true);}
        if case==3 {put(&mut g,&BAN,THEM);}
        assert!(offered(&mut g).is_empty(),"case {case}");
    }
}
#[test]
fn losing_the_recipients_abilities_does_not_remove_an_external_permission_but_losing_its_source_does() {
    for source_loses in [false,true] {
        let mut g=board();let source=put(&mut g,&RULE,YOU);let creature=put(&mut g,&CREATURE,YOU);
        cx(&mut g).add_effect(source,if source_loses {source}else{creature},EffectKind::LoseAllAbilities,Duration::EndOfTurn);
        assert_eq!(offered(&mut g).contains(&creature),!source_loses);
    }
}
#[test]
fn permissions_can_allow_only_one_defender_and_other_declarations_are_rejected() {
    let mut g=board();put(&mut g,&ONLY,YOU);let creature=put(&mut g,&CREATURE,YOU);assert_eq!(offered(&mut g),vec![creature]);
    let r=g.pending().cloned().unwrap();let RequestKind::DeclareAttackers {requirements,..}=&r.kind else {panic!()};
    assert!(requirements.bars(creature,Target::Player(THIRD)));assert!(!requirements.bars(creature,Target::Player(THEM)));
    assert!(g.answer(r.id,YOU,Answer::Attack(vec![(creature,Target::Player(THIRD))])).is_err());
    g.answer(r.id,YOU,Answer::Attack(vec![(creature,Target::Player(THEM))])).unwrap();g.run();
    assert!(g.state.attackers.iter().any(|a|a.id==creature));
}
#[test]
fn copied_source_texts_supply_the_permission() {
    let mut g=board();let source=put(&mut g,&DECOY,YOU);let creature=put(&mut g,&CREATURE,YOU);let card=g.cards.id_of(&RULE);
    cx(&mut g).set_copy(source,Copiable::of(card));assert_eq!(offered(&mut g),vec![creature]);
}
#[test]
fn an_absent_permission_source_does_not_change_a_summoning_sick_creatures_eligibility() {
    let mut g=board();put(&mut g,&CREATURE,YOU);assert!(offered(&mut g).is_empty());
}
