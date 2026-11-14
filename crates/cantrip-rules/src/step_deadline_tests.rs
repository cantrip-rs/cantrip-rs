//! CR 500.4, 500.11, 502.3, 611.2a and 800.4m, with forced step boundaries.
use super::*;
use crate::Setup;
use cantrip_api::Game as Rules;
const YOU:PlayerId=PlayerId(0);
const THEM:PlayerId=PlayerId(1);
fn upkeep_without_flying(q:&dyn Query, source:ObjectId, event:&Event)->Option<Fired> {
    if matches!(event,Event::StepBegan {step:Step::Upkeep}) && !q.chars(source).has(Keyword::Flying) {Fired::PLAIN} else {None}
}
static PERMANENT:CardDef=CardDef {name:"Step deadline probe",types:TypeSet::of(&[CardType::Creature]),power:Some(2),toughness:Some(2),
    abilities:&[triggered(when(EventMask::STEP_BEGAN,upkeep_without_flying),|_,_|Ok(()),"At the beginning of each upkeep, if this lacks flying, do nothing.")],..CardDef::BLANK};
static LAND:CardDef=CardDef {name:"Step deadline exiled land",types:TypeSet::of(&[CardType::Land]),..CardDef::BLANK};
fn board()->(Game,ObjectId) {
    let cards=Box::leak(Box::new(CardTable::new([&PERMANENT,&LAND])));
    let mut g=Game::new(cards,&Setup {decks:vec![vec![],vec![]],..Default::default()});
    while !g.state.agenda.is_empty(){g.state.frame_remove(0);}
    g.state.frame_push(Frame::new(FrameKind::BeginStep));
    g.state.update_turn(|t|{t.number=1;t.active=YOU;t.priority=YOU;t.set_step(Step::PrecombatMain);});
    let id=g.state.push_obj(Obj::new(cards.id_of(&PERMANENT),YOU,Zone::BATTLEFIELD));g.state.zone_push(Zone::BATTLEFIELD,id);g.state.see_statics();
    (g,id)
}
fn cx(g:&mut Game)->Cx<'_>{Cx {g,frame:0,cursor:0,mana_source:None,entering:None,lineage:Vec::new(),damage:None,asked_at:None,doer:None,producing:None}}
fn step(g:&mut Game, player:PlayerId, step:Step) {
    g.state.update_turn(|t|{t.active=player;t.priority=player;t.set_step(step);});crate::settle::step_began(&mut cx(g),step);
}
fn upkeep(player:Option<PlayerId>)->Duration {Duration::UntilNextStepOf {step:Step::Upkeep,player}}
fn add(g:&mut Game,id:ObjectId,duration:Duration){cx(g).add_effect(id,id,EffectKind::AddKeywords(Keywords::of(&[Keyword::Flying])),duration);}
#[test]
fn step_deadlines_expire_before_beginning_triggers_and_revoke_play_permissions() {
    let (mut g,id)=board();add(&mut g,id,upkeep(Some(YOU)));
    let card=g.state.push_obj(Obj::new(g.cards.id_of(&LAND),YOU,Zone::exile(YOU)));g.state.zone_push(Zone::exile(YOU),card);
    cx(&mut g).allow_play(card,YOU,PlayGrant {lands:true,expiry:Expiry::UntilYourNextStep(Step::Upkeep),..PlayGrant::CAST});
    assert!(actions::priority_options(&mut cx(&mut g),YOU).iter().any(|o|o.source==Some(card)&&o.kind==ActionKind::PlayLand));
    step(&mut g,THEM,Step::Upkeep);assert!(g.chars(id).has(Keyword::Flying));assert_eq!(g.state.permissions.len(),1);
    step(&mut g,YOU,Step::Draw);assert!(g.chars(id).has(Keyword::Flying));
    step(&mut g,YOU,Step::Upkeep);assert!(!g.chars(id).has(Keyword::Flying));assert!(g.state.permissions.is_empty());
    assert!(g.state.triggers.iter().any(|t|t.source==id),"the beginning trigger sees the expired ability");
}
#[test]
fn an_effect_created_during_the_named_step_waits_for_another_matching_beginning() {
    let (mut g,id)=board();step(&mut g,YOU,Step::Upkeep);add(&mut g,id,upkeep(Some(YOU)));
    next_step(&mut g);assert_eq!(g.state.turn.step(),Step::Draw);assert!(g.chars(id).has(Keyword::Flying));
    step(&mut g,THEM,Step::Upkeep);assert!(g.chars(id).has(Keyword::Flying));
    step(&mut g,YOU,Step::Upkeep);assert!(!g.chars(id).has(Keyword::Flying));
}
#[test]
fn through_next_upkeep_lasts_through_it_then_settles_control_before_the_following_step() {
    for end_turn in [false,true] {
        let (mut g,id)=board();step(&mut g,YOU,Step::Upkeep);
        cx(&mut g).add_effect(id,id,EffectKind::Control(THEM),Duration::UntilEndOfNextStepOf {step:Step::Upkeep,player:Some(YOU)});
        ops::settle_lasting_and_control(&mut cx(&mut g));assert_eq!(g.state.obj(id).controller,THEM);
        next_step(&mut g);assert_eq!(g.state.obj(id).controller,THEM,"the upkeep that created it wasn't its next upkeep");
        step(&mut g,THEM,Step::Upkeep);assert_eq!(g.state.obj(id).controller,THEM);
        step(&mut g,YOU,Step::Upkeep);assert_eq!(g.state.effects[0].duration,Duration::EndOfStep);assert_eq!(g.state.obj(id).controller,THEM);
        g.state.update_turn(|t|t.ending=end_turn);next_step(&mut g);
        assert_eq!(g.state.obj(id).controller,YOU);assert!(g.state.effects.is_empty());
        assert_eq!(g.state.turn.step(),if end_turn {Step::Cleanup}else{Step::Draw});
    }
}
#[test]
fn skipped_steps_do_not_expire_deadlines_and_departed_players_expire_at_their_would_be_turn() {
    let (mut g,id)=board();add(&mut g,id,upkeep(Some(YOU)));
    g.state.update_turn(|t|{t.set_step(Step::Untap);t.ending=true;});next_step(&mut g);
    assert!(g.chars(id).has(Keyword::Flying),"ending the turn skipped its upkeep");
    turn_would_have_begun(&mut g.state,THEM);assert!(g.chars(id).has(Keyword::Flying));
    turn_would_have_begun(&mut g.state,YOU);assert!(!g.chars(id).has(Keyword::Flying));
}
#[test]
fn counted_untap_durations_follow_the_permanents_controller_after_control_changes() {
    let (mut g,id)=board();g.state.update_obj(id,|o|o.tapped=true);
    cx(&mut g).add_effect(id,id,EffectKind::DoesntUntap,Duration::DuringControllersNextUntapSteps(2));
    untap_step(&mut cx(&mut g),THEM).unwrap();assert_eq!(g.state.effects[0].duration,Duration::DuringControllersNextUntapSteps(2));
    untap_step(&mut cx(&mut g),YOU).unwrap();assert!(g.state.obj(id).tapped);assert_eq!(g.state.effects[0].duration,Duration::DuringControllersNextUntapSteps(1));
    cx(&mut g).add_effect(id,id,EffectKind::Control(THEM),Duration::Indefinite);ops::settle_lasting_and_control(&mut cx(&mut g));
    cx(&mut g).add_effect(id,id,EffectKind::LoseAllAbilities,Duration::EndOfTurn);
    untap_step(&mut cx(&mut g),YOU).unwrap();assert!(g.state.obj(id).tapped);
    untap_step(&mut cx(&mut g),THEM).unwrap();assert!(g.state.obj(id).tapped);assert!(!g.state.effects.iter().any(|e|e.kind==EffectKind::DoesntUntap));
    untap_step(&mut cx(&mut g),THEM).unwrap();assert!(!g.state.obj(id).tapped);
}
#[test]
fn independent_untap_effects_advance_together_even_if_already_untapped_and_rollback_restores_them() {
    let (mut g,id)=board();
    cx(&mut g).add_effect(id,id,EffectKind::DoesntUntap,Duration::DuringControllersNextUntapSteps(0));assert!(g.state.effects.is_empty());
    for n in [1,2] {cx(&mut g).add_effect(id,id,EffectKind::DoesntUntap,Duration::DuringControllersNextUntapSteps(n));}
    let mark=g.state.mark();let log=g.log.len();
    untap_step(&mut cx(&mut g),YOU).unwrap();assert_eq!(g.state.effects.len(),1);assert_eq!(g.state.effects[0].duration,Duration::DuringControllersNextUntapSteps(1));
    g.state.rollback(mark);g.log.truncate(log);assert_eq!(g.state.effects.len(),2);assert_eq!(g.state.effects[1].duration,Duration::DuringControllersNextUntapSteps(2));
    ops::tap(&mut cx(&mut g),id);untap_step(&mut cx(&mut g),YOU).unwrap();assert!(g.state.obj(id).tapped);
    untap_step(&mut cx(&mut g),YOU).unwrap();assert!(g.state.obj(id).tapped);
    untap_step(&mut cx(&mut g),YOU).unwrap();assert!(!g.state.obj(id).tapped);
}
