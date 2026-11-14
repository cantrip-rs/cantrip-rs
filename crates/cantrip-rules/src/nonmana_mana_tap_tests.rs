//! CR 106.12/603.3/605.4: ordinary mana-tap observers wait for priority;
//! triggered mana abilities resolve immediately. Manabarbs and Burning Earth.
use crate::{Cx, Game, Setup};
use cantrip_api::*;
use cantrip_api::Game as Rules;
use cantrip_state::{Frame, FrameKind};
const YOU:PlayerId=PlayerId(0);
fn when_land(q:&dyn Query,_:ObjectId,event:&Event)->Option<Fired>{
    match *event {Event::TappedForMana {permanent,player,..} if q.chars(permanent).is(CardType::Land)=>Fired::about(Target::Player(player)),_=>None}
}
fn hurt(g:&mut dyn cantrip_api::Game,item:&StackItem)->Result<(),Suspend>{g.damage(item.object,item.subject,1);Ok(())}
const DAMAGE:AbilityDef=triggered(when(EventMask::TAPPED_FOR_MANA,when_land),hurt,"Whenever a player taps a land for mana, deal 1 damage to that player.");
static FILL:CardDef=CardDef {name:"Mana observer filler",types:TypeSet::of(&[CardType::Land]),..CardDef::BLANK};
static LAND:CardDef=CardDef {name:"Mana observer land",abilities:&[mana_ability(TAP,add(Mana::G),"{T}: Add {G}.")],..FILL};
static ARTIFACT:CardDef=CardDef {name:"Mana observer artifact",types:TypeSet::of(&[CardType::Artifact]),..LAND};
static BARBS:CardDef=CardDef {name:"Mana observer damage",types:TypeSet::of(&[CardType::Enchantment]),abilities:&[DAMAGE],..CardDef::BLANK};
static EXTRA:CardDef=CardDef {name:"Mana observer extra mana",abilities:&[triggered(when(EventMask::TAPPED,when_land),|g,item|{g.add_mana(item.subject.player().unwrap(),Mana::G,1);Ok(())},"Whenever a player taps a land for mana, add {G}.").mana_ability(TriggeredMana::One(Mana::G))],..BARBS};
static EXTRA_NEW_MASK:CardDef=CardDef {name:"Mana observer new-mask extra",abilities:&[triggered(when(EventMask::TAPPED_FOR_MANA,when_land),|g,item|{g.add_mana(item.subject.player().unwrap(),Mana::G,1);Ok(())},"Whenever a player taps a land for mana, add {G}.").mana_ability(TriggeredMana::One(Mana::G))],..BARBS};
static ZERO:CardDef=CardDef {name:"Mana observer zero replacement",abilities:&[],replacements:&[replacement(ReplacementKind::ManaProduction,|_,_,p|matches!(p,Proposal::ManaProduction(_)),|_,_,p|{if let Proposal::ManaProduction(m)=p {m.mana=ManaPool::default();}},"Produces no mana instead.")],..BARBS};
static HAND_OBSERVER:CardDef=CardDef {name:"Mana observer in hand",abilities:&[DAMAGE.functions_in(ZoneKinds::of(&[ZoneKind::Hand]))],..BARBS};
static GIFT:CardDef=CardDef {name:"Mana observer gift",abilities:&[],delayed:&[DAMAGE],..BARBS};
static SPELL:CardDef=CardDef {name:"Mana observer payment spell",types:TypeSet::of(&[CardType::Instant]),has_cost:true,cost:mana("{G}{G}"),spell:spell(&[],|_,_|Ok(())),..CardDef::BLANK};
fn board()->Game {
    let cards=Box::leak(Box::new(CardTable::new([&FILL,&LAND,&ARTIFACT,&BARBS,&EXTRA,&EXTRA_NEW_MASK,&ZERO,&GIFT,&HAND_OBSERVER,&SPELL])));let filler=cards.id_of(&FILL);
    let mut g=Game::new(cards,&Setup {decks:vec![vec![filler;20],vec![filler;20]],shuffle:false,..Default::default()});
    while !g.state.agenda.is_empty(){g.state.frame_remove(0);}g.state.frame_push(Frame::new(FrameKind::Priority));
    g.state.update_turn(|t|{t.number=1;t.active=YOU;t.priority=YOU;t.set_step(Step::PrecombatMain);});g
}
fn put(g:&mut Game,def:&'static CardDef,zone:Zone)->ObjectId{let id=g.state.push_obj(Obj::new(g.cards.id_of(def),YOU,zone));g.state.zone_push(zone,id);g.state.see_statics();id}
fn cx(g:&mut Game)->Cx<'_>{Cx {g,frame:0,cursor:0,mana_source:None,entering:None,lineage:Vec::new(),damage:None,asked_at:None,doer:None,producing:None}}
fn activate(g:&mut Game,source:ObjectId){g.run();let r=g.pending().cloned().unwrap();let RequestKind::Priority {options}=&r.kind else {panic!()};let opt=options.iter().find(|o|o.source==Some(source)&&o.kind==ActionKind::ActivateAbility).unwrap();g.answer(r.id,r.player,Answer::Act(Action::of(opt))).unwrap();g.run();}
fn resolve(g:&mut Game){for _ in 0..20 {g.run();if g.state.stack.is_empty(){return}let r=g.pending().cloned().unwrap();assert!(matches!(r.kind,RequestKind::Priority {..}));g.answer(r.id,r.player,Answer::Pass).unwrap();}panic!("unresolved");}
#[test]
fn mana_triggers_finish_immediately_and_the_ordinary_trigger_uses_the_stack_once() {
    for extra in [&EXTRA,&EXTRA_NEW_MASK] {
        let mut g=board();put(&mut g,&BARBS,Zone::BATTLEFIELD);put(&mut g,extra,Zone::BATTLEFIELD);let land=put(&mut g,&LAND,Zone::BATTLEFIELD);
        activate(&mut g,land);assert_eq!(g.state.player(YOU).pool.get(Mana::G),2);assert_eq!(g.state.stack.len(),1);assert_eq!(g.state.player(YOU).life,20);
        resolve(&mut g);assert_eq!(g.state.player(YOU).life,19);
    }
}
#[test]
fn ordinary_tapping_and_artifact_mana_do_not_trigger_a_land_observer() {
    let mut g=board();put(&mut g,&BARBS,Zone::BATTLEFIELD);let land=put(&mut g,&LAND,Zone::BATTLEFIELD);let artifact=put(&mut g,&ARTIFACT,Zone::BATTLEFIELD);
    crate::ops::tap(&mut cx(&mut g),land);activate(&mut g,artifact);
    assert!(g.state.stack.is_empty());assert_eq!(g.state.player(YOU).life,20);
}
#[test]
fn zero_production_still_triggers_ordinary_observers_but_no_additional_mana_trigger() {
    let mut g=board();put(&mut g,&BARBS,Zone::BATTLEFIELD);put(&mut g,&EXTRA,Zone::BATTLEFIELD);put(&mut g,&ZERO,Zone::BATTLEFIELD);let land=put(&mut g,&LAND,Zone::BATTLEFIELD);
    crate::actions::activate_mana(&mut cx(&mut g),YOU,land,0,None,false,&[]).unwrap();g.run();
    assert!(g.state.obj(land).tapped);assert!(g.state.player(YOU).pool.is_empty());assert_eq!(g.state.stack.len(),1);
    resolve(&mut g);assert_eq!(g.state.player(YOU).life,19);
}
#[test]
fn a_granted_observer_is_seen_by_the_trigger_guard_and_a_later_ability_loss_removes_it() {
    for lost in [false,true] {
        let mut g=board();let holder=put(&mut g,&GIFT,Zone::BATTLEFIELD);let land=put(&mut g,&LAND,Zone::BATTLEFIELD);let ability=AbilityRef::delayed(g.cards.id_of(&GIFT),0);
        cx(&mut g).add_effect(holder,holder,EffectKind::AddAbility(ability),Duration::EndOfTurn);
        if lost {cx(&mut g).add_effect(holder,holder,EffectKind::LoseAllAbilities,Duration::EndOfTurn);}
        activate(&mut g,land);assert_eq!(g.state.stack.len(),if lost {0}else{1});resolve(&mut g);assert_eq!(g.state.player(YOU).life,if lost {20}else{19});
    }
}
#[test]
fn a_mana_payment_queues_damage_until_casting_has_finished() {
    let mut g=board();put(&mut g,&BARBS,Zone::BATTLEFIELD);put(&mut g,&EXTRA,Zone::BATTLEFIELD);let land=put(&mut g,&LAND,Zone::BATTLEFIELD);let spell=put(&mut g,&SPELL,Zone::hand(YOU));g.run();
    let r=g.pending().cloned().unwrap();let RequestKind::Priority {options}=&r.kind else {panic!()};let opt=options.iter().find(|o|o.source==Some(spell)&&o.kind==ActionKind::CastSpell).unwrap();g.answer(r.id,YOU,Answer::Act(Action::of(opt))).unwrap();
    for _ in 0..30 {
        g.run();let r=g.pending().cloned().unwrap();if matches!(r.kind,RequestKind::Priority {..}) {break;}
        assert_eq!(g.state.player(YOU).life,20);
        let answer=match &r.kind {
            RequestKind::PayCosts {announcing:true,..}=>Answer::Announce(0),
            RequestKind::PayCosts {options,..}=>{let opt=if !g.state.obj(land).tapped {options.iter().find(|o|o.source==Some(land)&&o.kind==ActionKind::ActivateAbility)}else{None}.or_else(||[ActionKind::AssignMana,ActionKind::PayCost,ActionKind::ConfirmCasting].into_iter().find_map(|k|options.iter().find(|o|o.kind==k))).unwrap();Answer::Act(Action::of(opt))},
            RequestKind::ChooseCostPayment {..}=>Answer::CostPayment(0),other=>panic!("{other:?}"),
        };g.answer(r.id,r.player,answer).unwrap();
    }
    assert_eq!(g.state.stack.len(),2);assert_eq!(g.state.player(YOU).life,20);resolve(&mut g);assert_eq!(g.state.player(YOU).life,19);
}

#[test]
fn the_guard_is_about_objects_in_the_game_not_cards_anywhere_in_the_table() {
    let mut g=board();let land=put(&mut g,&LAND,Zone::BATTLEFIELD);
    assert!(!crate::settle::mana_tap_watched(&cx(&mut g)));
    activate(&mut g,land);assert!(g.state.stack.is_empty());assert_eq!(g.state.player(YOU).pool.get(Mana::G),1);
}
#[test]
fn observers_functioning_in_other_zones_are_not_skipped_by_the_guard() {
    let mut g=board();put(&mut g,&HAND_OBSERVER,Zone::hand(YOU));let land=put(&mut g,&LAND,Zone::BATTLEFIELD);
    assert!(crate::settle::mana_tap_watched(&cx(&mut g)));
    activate(&mut g,land);assert_eq!(g.state.stack.len(),1);resolve(&mut g);assert_eq!(g.state.player(YOU).life,19);
}
