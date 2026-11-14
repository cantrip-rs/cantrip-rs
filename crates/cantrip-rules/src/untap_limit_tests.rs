//! CR 502.3: intersect simultaneous quotas, then untap once. Smoke, Damping
//! Field, and Winter Orb are represented by fixture definitions, not card IDs.
use crate::{Cx, Game, Setup};
use cantrip_api::*;
use cantrip_api::Game as Rules;
use cantrip_state::{Frame, FrameKind};
const YOU:PlayerId=PlayerId(0);
const THEM:PlayerId=PlayerId(1);
fn all(_: &dyn Query,_:&Asker,_:PlayerId)->bool{true}
fn untapped(q:&dyn Query,a:&Asker,_:PlayerId)->bool{!q.state().obj(a.source).tapped}
fn lands(q:&dyn Query,_:&Asker,id:ObjectId)->bool{q.chars(id).is(CardType::Land)}
fn creatures(q:&dyn Query,_:&Asker,id:ObjectId)->bool{q.chars(id).is(CardType::Creature)}
fn artifacts(q:&dyn Query,_:&Asker,id:ObjectId)->bool{q.chars(id).is(CardType::Artifact)}
static LAND:CardDef=CardDef {name:"Limit land",types:TypeSet::of(&[CardType::Land]),..CardDef::BLANK};
static BEAR:CardDef=CardDef {name:"Limit bear",types:TypeSet::of(&[CardType::Creature]),power:Some(2),toughness:Some(2),..CardDef::BLANK};
static ANIMATED:CardDef=CardDef {name:"Limit animated land",types:TypeSet::of(&[CardType::Land,CardType::Creature]),..BEAR};
static ARTIFACT:CardDef=CardDef {name:"Limit artifact",types:TypeSet::of(&[CardType::Artifact]),..CardDef::BLANK};
static ORB:CardDef=CardDef {name:"Limit conditional land rule",types:TypeSet::of(&[CardType::Artifact]),untap_limits:&[UntapLimit {applies:untapped,permanents:lands,amount:1}],..CardDef::BLANK};
static SMOKE:CardDef=CardDef {name:"Limit creature rule",types:TypeSet::of(&[CardType::Enchantment]),untap_limits:&[UntapLimit {applies:all,permanents:creatures,amount:1}],..CardDef::BLANK};
static FIELD:CardDef=CardDef {name:"Limit artifact rule",untap_limits:&[UntapLimit {applies:all,permanents:artifacts,amount:1}],..SMOKE};
static ZERO:CardDef=CardDef {name:"Limit zero land rule",untap_limits:&[UntapLimit {applies:all,permanents:lands,amount:0}],..SMOKE};
static FROZEN:CardDef=CardDef {name:"Limit frozen land",keywords:Keywords::of(&[Keyword::DoesNotUntap]),..LAND};
static CANT:CardDef=CardDef {name:"Limit absolute frozen land",cant_become_untapped:&[|_,a,id|a.source==id],..LAND};
static OPTIONAL:CardDef=CardDef {name:"Limit optional untap land",may_not_untap:true,..LAND};
static SKIN:CardDef=CardDef {name:"Limit other untap",untap_in_others_untap_steps:&[|_,a,id|id==a.source],..LAND};
fn board()->Game {
    let cards=Box::leak(Box::new(CardTable::new([&LAND,&BEAR,&ANIMATED,&ARTIFACT,&ORB,&SMOKE,&FIELD,&ZERO,&FROZEN,&CANT,&SKIN,&OPTIONAL])));
    let mut g=Game::new(cards,&Setup {decks:vec![vec![],vec![]],..Default::default()});
    while !g.state.agenda.is_empty(){g.state.frame_remove(0);}
    g.state.frame_push(Frame::new(FrameKind::BeginStep));
    g.state.update_turn(|t|{t.number=2;t.active=YOU;t.priority=YOU;t.set_step(Step::Untap);t.began=false;});g
}
fn put(g:&mut Game,def:&'static CardDef,p:PlayerId,tapped:bool)->ObjectId {
    let id=g.state.push_obj(Obj::new(g.cards.id_of(def),p,Zone::BATTLEFIELD));g.state.zone_push(Zone::BATTLEFIELD,id);
    g.state.update_obj(id,|o|o.tapped=tapped);g.state.see_statics();id
}
fn cx(g:&mut Game)->Cx<'_>{Cx {g,frame:0,cursor:0,mana_source:None,entering:None,lineage:Vec::new(),damage:None,asked_at:None,doer:None,producing:None}}
fn start(g:&mut Game)->Request {g.run();g.pending().cloned().unwrap()}
fn choose(g:&mut Game,r:&Request,ids:Vec<ObjectId>){g.answer(r.id,r.player,Answer::Cards(ids)).unwrap();g.run();}
fn direct(g:&mut Game){crate::turn::untap_step(&mut cx(g),YOU).expect("no choice needed");}
#[test]
fn intersecting_limits_accept_both_maximal_shapes_and_reject_incomplete_or_excess_answers() {
    for animated_only in [false,true] {
        let mut g=board();put(&mut g,&ORB,THEM,false);put(&mut g,&SMOKE,THEM,false);
        let land=put(&mut g,&LAND,YOU,true);let bear=put(&mut g,&BEAR,YOU,true);let animated=put(&mut g,&ANIMATED,YOU,true);
        let other=put(&mut g,&ARTIFACT,YOU,true);let r=start(&mut g);
        assert_eq!(r.player,YOU);let RequestKind::ChooseCards {purpose:Purpose::Untap,cards,..}=&r.kind else {panic!("{:?}",r.kind)};
        assert_eq!(cards,&vec![land,bear,animated]);assert!(g.state.obj(other).tapped,"no untaps before all choices");
        assert!(g.answer(r.id,r.player,Answer::Cards(vec![])).is_err());
        assert!(g.answer(r.id,r.player,Answer::Cards(vec![land])).is_err(),"the unrelated creature can still untap");
        assert!(g.answer(r.id,r.player,Answer::Cards(vec![land,animated])).is_err());
        assert!(g.answer(r.id,r.player,Answer::Cards(vec![bear,animated])).is_err());
        choose(&mut g,&r,if animated_only {vec![animated]}else{vec![land,bear]});
        assert_eq!(g.state.obj(animated).tapped,!animated_only);
        assert_eq!(g.state.obj(land).tapped,animated_only);assert_eq!(g.state.obj(bear).tapped,animated_only);assert!(!g.state.obj(other).tapped);
    }
}
#[test]
fn a_tapped_conditional_source_does_not_turn_on_mid_untap_and_lost_abilities_turn_it_off() {
    for lost in [false,true] {
        let mut g=board();let orb=put(&mut g,&ORB,THEM,!lost);
        let a=put(&mut g,&LAND,YOU,true);let b=put(&mut g,&LAND,YOU,true);
        if lost {cx(&mut g).add_effect(orb,orb,EffectKind::LoseAllAbilities,Duration::Indefinite);}
        direct(&mut g);assert!(!g.state.obj(a).tapped&&!g.state.obj(b).tapped);
    }
    let mut g=board();let orb=put(&mut g,&ORB,YOU,true);let a=put(&mut g,&LAND,YOU,true);let b=put(&mut g,&LAND,YOU,true);
    direct(&mut g);assert!(!g.state.obj(orb).tapped&&!g.state.obj(a).tapped&&!g.state.obj(b).tapped);
}
#[test]
fn duplicate_limits_do_not_add_and_artifact_creatures_count_for_both() {
    let mut g=board();put(&mut g,&SMOKE,THEM,false);put(&mut g,&SMOKE,THEM,false);put(&mut g,&FIELD,THEM,false);
    let bear=put(&mut g,&BEAR,YOU,true);let artifact=put(&mut g,&ARTIFACT,YOU,true);let dual=put(&mut g,&ARTIFACT,YOU,true);
    let timestamp=g.state.timestamp();g.state.effect_push(ContinuousEffect {source:dual,controller:YOU,timestamp,affected:dual,kind:EffectKind::AddTypes(TypeSet::of(&[CardType::Creature])),duration:Duration::Indefinite,words:Words::NONE});
    let timestamp=g.state.timestamp();g.state.effect_push(ContinuousEffect {source:dual,controller:YOU,timestamp,affected:dual,kind:EffectKind::SetPt(2,2),duration:Duration::Indefinite,words:Words::NONE});
    let r=start(&mut g);assert!(g.answer(r.id,r.player,Answer::Cards(vec![bear,dual])).is_err());
    choose(&mut g,&r,vec![dual]);assert!(!g.state.obj(dual).tapped);assert!(g.state.obj(bear).tapped&&g.state.obj(artifact).tapped);
}
#[test]
fn impossible_untaps_do_not_consume_a_slot_but_stun_replacements_do() {
    let mut g=board();put(&mut g,&ORB,THEM,false);let frozen=put(&mut g,&FROZEN,YOU,true);let cant=put(&mut g,&CANT,YOU,true);let free=put(&mut g,&LAND,YOU,true);
    direct(&mut g);assert!(g.state.obj(frozen).tapped&&g.state.obj(cant).tapped);assert!(!g.state.obj(free).tapped);
    let mut g=board();put(&mut g,&ORB,THEM,false);let stun=put(&mut g,&LAND,YOU,true);let other=put(&mut g,&LAND,YOU,true);
    crate::counters::put(&mut cx(&mut g),stun,CounterKind::Stun,1,YOU,crate::counters::How::Effect);
    let r=start(&mut g);choose(&mut g,&r,vec![stun]);assert!(g.state.obj(stun).tapped&&g.state.obj(other).tapped);assert_eq!(g.state.obj(stun).counters(CounterKind::Stun),0);
}
#[test]
fn zero_limits_other_players_and_effect_untaps_stay_separate() {
    let mut g=board();put(&mut g,&ZERO,THEM,false);let mine=put(&mut g,&LAND,YOU,true);let theirs=put(&mut g,&SKIN,THEM,true);let bear=put(&mut g,&BEAR,YOU,true);
    direct(&mut g);assert!(g.state.obj(mine).tapped);assert!(!g.state.obj(theirs).tapped&&!g.state.obj(bear).tapped);
    cx(&mut g).untap(mine);assert!(!g.state.obj(mine).tapped);
}
#[test]
fn a_limit_follows_a_copy_and_undo_restores_a_simultaneous_untap() {
    let mut g=board();let source=put(&mut g,&ARTIFACT,THEM,false);let orb=g.cards.id_of(&ORB);cx(&mut g).set_copy(source,Copiable::of(orb));
    let a=put(&mut g,&LAND,YOU,true);let b=put(&mut g,&LAND,YOU,true);let r=start(&mut g);
    let node=g.checkpoint();choose(&mut g,&r,vec![a]);assert!(!g.state.obj(a).tapped&&g.state.obj(b).tapped);
    g.goto(node);assert!(g.state.obj(a).tapped&&g.state.obj(b).tapped);let r=g.pending().cloned().unwrap();choose(&mut g,&r,vec![b]);
    assert!(g.state.obj(a).tapped&&!g.state.obj(b).tapped);
}

#[test]
fn optional_untaps_are_decided_before_quotas_and_no_permanent_untaps_between_questions() {
    for leave in [false,true] {
        let mut g=board();put(&mut g,&ORB,THEM,false);let optional=put(&mut g,&OPTIONAL,YOU,true);let other=put(&mut g,&LAND,YOU,true);
        let r=start(&mut g);assert!(matches!(r.kind,RequestKind::May {..}));
        g.answer(r.id,YOU,if leave {Answer::No}else{Answer::Yes}).unwrap();g.run();
        if leave {assert!(g.state.obj(optional).tapped);assert!(!g.state.obj(other).tapped);}
        else {
            let r=g.pending().cloned().unwrap();assert!(matches!(r.kind,RequestKind::ChooseCards {purpose:Purpose::Untap,..}));
            assert!(g.state.obj(optional).tapped&&g.state.obj(other).tapped);
            choose(&mut g,&r,vec![optional]);assert!(!g.state.obj(optional).tapped&&g.state.obj(other).tapped);
        }
    }
}
