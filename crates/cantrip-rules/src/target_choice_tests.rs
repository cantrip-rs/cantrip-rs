//! Opponents choose targets for Cuombajj Witches, Preacher and Nova Pentacle.
use crate::{Cx, Game, Setup, actions, ops, resolve};
use cantrip_api::*;
use cantrip_state::{Frame, FrameKind};
const A: PlayerId=PlayerId(0);const B: PlayerId=PlayerId(1);const C: PlayerId=PlayerId(2);
fn opponents(q:&dyn Query,a:&Asker)->Vec<PlayerId>{q.state().seats().filter(|&p|p!=a.controller).collect()}
fn fixed(_: &dyn Query,_:&Asker)->PlayerId{B}
fn any(q:&dyn Query,_:&Asker,t:Target)->bool{match t{Target::Player(_)=>true,Target::Object(o)=>q.is_on_battlefield(o)&&q.chars(o).is(CardType::Creature),_=>false}}
fn their_creature(q:&dyn Query,_:&Asker,t:Target)->bool{t.object().is_some_and(|o|q.is_on_battlefield(o)&&q.chars(o).is(CardType::Creature)&&Some(q.controller(o))==q.target_chooser())}
fn damage(g:&mut dyn cantrip_api::Game,item:&StackItem)->Result<(),Suspend>{let targets=g.state().targets(item).to_vec();for(i,t)in targets.into_iter().enumerate(){if g.target_legal(item,i){g.damage(item.object,t,1);}}Ok(())}
fn noop(_: &mut dyn cantrip_api::Game,_:&StackItem)->Result<(),Suspend>{Ok(())}
static FILLER:CardDef=CardDef{name:"Target chooser filler",types:TypeSet::of(&[CardType::Land]),..CardDef::BLANK};
static CREATURE:CardDef=CardDef{name:"Target chooser creature",types:TypeSet::of(&[CardType::Creature]),power:Some(2),toughness:Some(2),..CardDef::BLANK};
static HEXPROOF:CardDef=CardDef{name:"Target chooser hexproof creature",keywords:Keywords::of(&[Keyword::Hexproof]),..CREATURE};
static WITCHES:CardDef=CardDef{name:"Target chooser two players",abilities:&[ability(TAP,damage,"Two players choose a target each.").targets(&[target(any),target(any).chosen_by_one_of(opponents)])],..CREATURE};
static PREACHER:CardDef=CardDef{name:"Target chooser controller restriction",abilities:&[ability(TAP,noop,"An opponent chooses a creature they control.").targets(&[target(their_creature).chosen_by_one_of(opponents)])],..CREATURE};
static FIXED:CardDef=CardDef{name:"Target chooser specified player",abilities:&[ability(TAP,noop,"The named player chooses.").targets(&[target(any).chosen_by(fixed)])],..CREATURE};
static DOUBLE:CardDef=CardDef{name:"Target chooser two creatures",abilities:&[ability(TAP,noop,"One opponent chooses two creatures they control.").targets(&[target(their_creature).between(2,2).chosen_by_one_of(opponents)])],..CREATURE};
fn begins(_: &dyn Query,_:ObjectId,e:&Event)->Option<Fired>{matches!(e,Event::StepBegan{step:Step::PrecombatMain}).then_some(Fired::PLAIN.unwrap())}
static TRIGGER:CardDef=CardDef{name:"Target chooser triggered ability",types:TypeSet::of(&[CardType::Enchantment]),abilities:&[triggered(when(EventMask::STEP_BEGAN,begins),noop,"An opponent chooses a target.").targets(&[target(their_creature).chosen_by_one_of(opponents)])],..CardDef::BLANK};
fn fixed_c(_: &dyn Query,_:&Asker)->PlayerId{C}
fn gain_chooser(g:&mut dyn cantrip_api::Game,item:&StackItem)->Result<(),Suspend>{let p=g.state().target_chooser(item,0).expect("this mode's retained chooser");g.gain_life(p,1);Ok(())}
static MODES:ModalDef=ModalDef{modes:&[mode(gain_chooser,"B chooses").targets(&[target(any).chosen_by(fixed)]),mode(gain_chooser,"C chooses").targets(&[target(any).chosen_by(fixed_c)])],min:2,max:2,escalate:None,repeat:false,allowed:None,when_paid:None,up_to_if:None,shared:None};
static MODAL:CardDef=CardDef{name:"Target chooser modal ability",abilities:&[modal_ability(TAP,&MODES,"Choose both")],..CREATURE};
fn slot_opponent(q:&dyn Query,_:&Asker)->PlayerId{q.slot_opponent().expect("this opponent's slot")}
static EACH:CardDef=CardDef{name:"Target chooser per opponent",abilities:&[ability(TAP,noop,"Each opponent chooses one of their creatures.").targets(&[TargetSpec{min:0,max:TargetSpec::EACH_OPPONENT,..target(their_creature).chosen_by(slot_opponent)}])],..CREATURE};
fn game(seats:usize)->Game{
 let cards=Box::leak(Box::new(CardTable::new([&FILLER,&CREATURE,&HEXPROOF,&WITCHES,&PREACHER,&FIXED,&DOUBLE,&TRIGGER,&MODAL,&EACH])));let fill=cards.id_of(&FILLER);
 let mut g=Game::new(cards,&Setup{decks:vec![vec![fill;20];seats],shuffle:false,..Default::default()});g.verify=true;
 while !g.state.agenda.is_empty(){g.state.frame_remove(0);}g.state.frame_push(Frame::new(FrameKind::Priority));
 g.state.update_turn(|t|{t.active=A;t.priority=A;t.number=2;t.set_step(Step::PrecombatMain);});g.state.update_player(A,|p|p.turn_began=1);g
}
fn put(g:&mut Game,d:&'static CardDef,p:PlayerId)->ObjectId{let id=g.state.push_obj(Obj::new(g.cards.id_of(d),p,Zone::BATTLEFIELD));g.state.zone_push(Zone::BATTLEFIELD,id);g.state.update_obj(id,|o|o.controlled_since=0);g.state.see_statics();id}
fn cx(g:&mut Game)->Cx<'_>{Cx{g,frame:0,cursor:0,mana_source:None,entering:None,lineage:vec![],damage:None,asked_at:None,doer:None,producing:None}}
fn pending(g:&mut Game)->Request{g.run();g.pending().cloned().expect("a request")}
fn answer(g:&mut Game,r:Request,a:Answer){g.answer(r.id,r.player,a).unwrap();}
fn activate(g:&mut Game,source:ObjectId){let r=pending(g);let RequestKind::Priority{ref options}=r.kind else{panic!("{:?}",r.kind)};let option=options.iter().find(|o|o.kind==ActionKind::ActivateAbility&&o.source==Some(source)).expect("activation offered");assert_eq!(option.targets_required,0,"the controller cannot preempt the other player's choice");let act=Action::of(option);answer(g,r,Answer::Act(act));}
fn targets(g:&mut Game,who:PlayerId,chosen:&[Target]){let r=pending(g);assert_eq!(r.player,who);assert!(matches!(r.kind,RequestKind::ChooseTarget{..}),"{:?}",r.kind);answer(g,r,Answer::Targets(chosen.to_vec()));}
#[test]
fn two_target_slots_are_answered_by_different_players_and_keep_the_original_controller(){
 let mut g=game(2);let source=put(&mut g,&WITCHES,A);activate(&mut g,source);targets(&mut g,A,&[Target::Player(B)]);targets(&mut g,B,&[Target::Player(A)]);
 pending(&mut g);let item=*g.state.stack.last().unwrap();assert_eq!(item.controller,A);assert_eq!(g.state.target_chooser(&item,1),Some(B));assert_eq!(g.state.target_chooser(&item,0),None);
 assert!(g.events().iter().filter(|r|matches!(r.event,Event::BecameTarget{..})).all(|r|matches!(r.event,Event::BecameTarget{controller,..} if controller==A)));
 for _ in 0..20{let r=pending(&mut g);if g.state.stack.is_empty(){break;}answer(&mut g,r,Answer::Pass);}
 assert_eq!((g.state.player(A).life,g.state.player(B).life),(19,19));
}
#[test]
fn an_opponent_cannot_target_their_hexproof_creature_with_your_ability(){
 let mut g=game(2);let source=put(&mut g,&WITCHES,A);let shielded=put(&mut g,&HEXPROOF,B);activate(&mut g,source);targets(&mut g,A,&[Target::Player(B)]);
 let r=pending(&mut g);assert_eq!(r.player,B);assert!(!r.may_cancel);let RequestKind::ChooseTarget{ref candidates,..}=r.kind else{panic!()};assert!(!candidates.contains(&Target::Object(shielded)));
}
#[test]
fn one_opponent_is_chosen_before_their_controlled_target_and_that_choice_is_frozen(){
 let mut g=game(3);let source=put(&mut g,&PREACHER,A);let b=put(&mut g,&CREATURE,B);let c=put(&mut g,&CREATURE,C);activate(&mut g,source);
 let r=pending(&mut g);assert_eq!(r.player,A);let RequestKind::ChoosePlayer{ref candidates,..}=r.kind else{panic!("{:?}",r.kind)};assert_eq!(candidates,&vec![B,C]);answer(&mut g,r,Answer::Player(B));
 let r=pending(&mut g);assert_eq!(r.player,B);let RequestKind::ChooseTarget{ref candidates,..}=r.kind else{panic!()};assert_eq!(candidates,&vec![Target::Object(b)]);assert!(!candidates.contains(&Target::Object(c)));answer(&mut g,r,Answer::Targets(vec![Target::Object(b)]));pending(&mut g);
 let item=*g.state.stack.last().unwrap();assert!(resolve::target_legal(g.cards,&g.state,g.events(),&item,0));g.state.update_obj(b,|o|o.controller=C);g.state.see_statics();assert!(!resolve::target_legal(g.cards,&g.state,g.events(),&item,0));assert_eq!(g.state.target_chooser(&item,0),Some(B));
}
#[test]
fn opponents_without_a_legal_target_cannot_be_selected_to_complete_the_slot(){
 let mut g=game(3);let source=put(&mut g,&PREACHER,A);put(&mut g,&HEXPROOF,B);let c=put(&mut g,&CREATURE,C);activate(&mut g,source);let r=pending(&mut g);assert_eq!(r.player,C);assert!(matches!(r.kind,RequestKind::ChooseTarget{..}));answer(&mut g,r,Answer::Targets(vec![Target::Object(c)]));
}
#[test]
fn availability_does_not_combine_two_different_opponents_single_creatures(){
 let mut g=game(3);let source=put(&mut g,&DOUBLE,A);put(&mut g,&CREATURE,B);put(&mut g,&CREATURE,C);
 assert!(!actions::priority_options(&mut cx(&mut g),A).iter().any(|o|o.kind==ActionKind::ActivateAbility&&o.source==Some(source)));
 put(&mut g,&CREATURE,B);assert!(actions::priority_options(&mut cx(&mut g),A).iter().any(|o|o.kind==ActionKind::ActivateAbility&&o.source==Some(source)));
}
#[test]
fn a_specified_player_chooses_without_changing_the_ability_controller(){let mut g=game(2);let source=put(&mut g,&FIXED,A);activate(&mut g,source);targets(&mut g,B,&[Target::Player(A)]);pending(&mut g);assert_eq!(g.state.stack.last().unwrap().controller,A);}
#[test]
fn copied_abilities_and_retarget_candidates_retain_the_original_chosen_opponent(){
 let mut g=game(3);let source=put(&mut g,&PREACHER,A);let b=put(&mut g,&CREATURE,B);let second=put(&mut g,&CREATURE,B);let c=put(&mut g,&CREATURE,C);activate(&mut g,source);let r=pending(&mut g);answer(&mut g,r,Answer::Player(B));targets(&mut g,B,&[Target::Object(b)]);pending(&mut g);
 let original=*g.state.stack.last().unwrap();ops::copy_ability(&mut cx(&mut g),Target::Ability(original.serial),C,false).unwrap();let copied=*g.state.stack.last().unwrap();assert_eq!(g.state.target_chooser(&copied,0),Some(B));
 let def=match g.cards.ability(copied.ability){AbilityDef::Activated(d)=>d,_=>panic!()};let a=Asker::new(source,C);
 let offered=actions::item_target_candidates(&cx(&mut g),&a,&def.targets[0],&[],&copied,None);assert!(offered.contains(&Target::Object(second)));assert!(!offered.contains(&Target::Object(c)));
}
#[test]
fn triggered_target_slots_use_their_own_chooser_and_record_it_with_the_item(){
 let mut g=game(2);let source=put(&mut g,&TRIGGER,A);let b=put(&mut g,&CREATURE,B);put(&mut g,&CREATURE,B);while !g.state.agenda.is_empty(){g.state.frame_remove(0);}g.state.frame_push(Frame::new(FrameKind::BeginStep));
 let r=pending(&mut g);assert_eq!(r.player,B);assert!(matches!(r.kind,RequestKind::ChooseTarget{..}),"{:?}",r.kind);answer(&mut g,r,Answer::Targets(vec![Target::Object(b)]));pending(&mut g);let item=*g.state.stack.last().unwrap();assert_eq!(item.object,source);assert_eq!(g.state.target_chooser(&item,0),Some(B));
}

#[test]
fn each_opponents_expanded_slot_retains_its_own_chosen_player() {
 let mut g=game(3);let source=put(&mut g,&EACH,A);let b=put(&mut g,&CREATURE,B);let c=put(&mut g,&CREATURE,C);activate(&mut g,source);
 targets(&mut g,B,&[Target::Object(b)]);targets(&mut g,C,&[Target::Object(c)]);pending(&mut g);let item=*g.state.stack.last().unwrap();
 assert_eq!(g.state.target_choosers(&item),&[Some(B),Some(C)]);assert!(resolve::target_legal(g.cards,&g.state,g.events(),&item,0));assert!(resolve::target_legal(g.cards,&g.state,g.events(),&item,1));
}
#[test]
fn modal_resolution_reads_each_modes_own_retained_player() {
 let mut g=game(3);let source=put(&mut g,&MODAL,A);activate(&mut g,source);
 let r=pending(&mut g);if matches!(r.kind,RequestKind::ChooseModes{..}){answer(&mut g,r,Answer::Modes(vec![0,1]));}
 targets(&mut g,B,&[Target::Player(A)]);targets(&mut g,C,&[Target::Player(A)]);
 for _ in 0..20{let r=pending(&mut g);if g.state.stack.is_empty(){break;}answer(&mut g,r,Answer::Pass);}
 assert_eq!((g.state.player(A).life,g.state.player(B).life,g.state.player(C).life),(20,21,21));
}
