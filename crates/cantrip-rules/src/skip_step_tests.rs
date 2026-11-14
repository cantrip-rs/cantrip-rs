//! Skipping a step skips its whole procedure (CR 103.8a, 500.11, 614.10).
use crate::{Cx, Flow, Game, Setup, turn};
use cantrip_api::*;
use cantrip_api::Game as Rules;
use cantrip_state::{Frame, FrameKind};
const YOU:PlayerId=PlayerId(0);const THEM:PlayerId=PlayerId(1);
fn all(_: &dyn Query,_:&Asker,_:PlayerId)->bool{true}
fn yours(_: &dyn Query,a:&Asker,p:PlayerId)->bool{a.controller==p}
static LAND:CardDef=CardDef {name:"Skip fixture land",types:TypeSet::of(&[CardType::Land]),..CardDef::BLANK};
static CREATURE:CardDef=CardDef {name:"Skip fixture creature",types:TypeSet::of(&[CardType::Creature]),power:Some(2),toughness:Some(2),delayed:&[triggered(when(EventMask::STEP_BEGAN,|_,_,_|None),|_,_|Ok(()),"At the beginning of the next draw step, do nothing.")],..CardDef::BLANK};
static PHASING:CardDef=CardDef {name:"Skip fixture phasing",keywords:Keywords::of(&[Keyword::Phasing]),..CREATURE};
static UNTAP:CardDef=CardDef {name:"Skip fixture Stasis",types:TypeSet::of(&[CardType::Enchantment]),skip_steps:&[SkipStep {applies:all,step:Step::Untap}],..CardDef::BLANK};
static DRAW:CardDef=CardDef {name:"Skip fixture Necropotence",skip_steps:&[SkipStep {applies:yours,step:Step::Draw}],..UNTAP};
static UPKEEP:CardDef=CardDef {name:"Skip fixture upkeep",skip_steps:&[SkipStep {applies:all,step:Step::Upkeep}],..UNTAP};
fn setup()->Game{
    let cards=Box::leak(Box::new(CardTable::new([&LAND,&CREATURE,&PHASING,&UNTAP,&DRAW,&UPKEEP])));let land=cards.id_of(&LAND);
    let mut g=Game::new(cards,&Setup{decks:vec![vec![land;20],vec![land;20]],shuffle:false,..Default::default()});
    while !g.state.agenda.is_empty(){g.state.frame_remove(0);}g.state.frame_push(Frame::new(FrameKind::BeginStep));
    g.state.update_turn(|t|{t.number=2;t.active=YOU;t.priority=YOU;});g
}
fn put(g:&mut Game,d:&'static CardDef,p:PlayerId)->ObjectId{let id=g.state.push_obj(Obj::new(g.cards.id_of(d),p,Zone::BATTLEFIELD));g.state.zone_push(Zone::BATTLEFIELD,id);g.state.see_statics();id}
fn cx(g:&mut Game)->Cx<'_>{Cx {g,frame:0,cursor:0,mana_source:None,entering:None,lineage:Vec::new(),damage:None,asked_at:None,doer:None,producing:None}}
fn begin(g:&mut Game,player:PlayerId,step:Step){g.state.update_turn(|t|{t.active=player;t.priority=player;t.set_step(step);});assert!(matches!(turn::begin_step(&mut cx(g)),Ok(Flow::Done)));}
fn began(g:&Game,step:Step)->bool{g.history().this_turn().iter().any(|r|matches!(r.event,Event::StepBegan{step:s} if s==step))}
#[test]
fn skipping_untap_still_begins_the_turn_but_preserves_untap_deadlines_and_actions(){
    let mut g=setup();put(&mut g,&UNTAP,YOU);let creature=put(&mut g,&PHASING,YOU);
    g.state.update_obj(creature,|o|o.tapped=true);g.state.update_player(YOU,|p|p.lands_played=1);
    for (kind,duration) in [(EffectKind::AddKeywords(Keywords::of(&[Keyword::Flying])),Duration::UntilNextTurnOf(YOU)),
        (EffectKind::AddKeywords(Keywords::of(&[Keyword::Trample])),Duration::UntilNextStepOf{step:Step::Untap,player:Some(YOU)}),
        (EffectKind::DoesntUntap,Duration::DuringControllersNextUntapSteps(2)),
        (EffectKind::AddKeywords(Keywords::of(&[Keyword::Haste])),Duration::UntilItsControllersNextUntapStep)] {cx(&mut g).add_effect(creature,creature,kind,duration);}
    begin(&mut g,YOU,Step::Untap);
    assert_eq!(g.state.player(YOU).turn_began,2);assert_eq!(g.state.player(YOU).lands_played,0);
    assert!(g.history().this_turn().iter().any(|r|matches!(r.event,Event::TurnBegan{player:YOU,..})));
    assert!(!g.chars(creature).has(Keyword::Flying));assert!(g.chars(creature).has(Keyword::Trample));
    assert!(g.state.obj(creature).marks.phased.is_none());assert!(g.state.obj(creature).tapped);
    assert!(g.state.effects.iter().any(|e|e.duration==Duration::DuringControllersNextUntapSteps(2)));
    assert!(g.chars(creature).has(Keyword::Haste));
    assert_eq!(g.state.agenda.len(),1,"no priority is offered for the skipped step");
}
#[test]
fn turn_bound_ability_loss_expires_before_deciding_whether_untap_is_skipped(){
    let mut g=setup();let source=put(&mut g,&UNTAP,YOU);let creature=put(&mut g,&CREATURE,YOU);g.state.update_obj(creature,|o|o.tapped=true);
    cx(&mut g).add_effect(source,source,EffectKind::LoseAllAbilities,Duration::UntilNextTurnOf(YOU));
    assert!(g.lost_all_abilities(source));begin(&mut g,YOU,Step::Untap);
    assert!(!g.lost_all_abilities(source));assert!(g.state.obj(creature).tapped);
}
#[test]
fn skipped_draw_has_no_draw_trigger_priority_or_deadline_expiry_and_only_applies_to_its_player(){
    let mut g=setup();let source=put(&mut g,&DRAW,YOU);let creature=put(&mut g,&CREATURE,YOU);
    cx(&mut g).add_effect(source,creature,EffectKind::AddKeywords(Keywords::of(&[Keyword::Flying])),Duration::UntilNextStepOf{step:Step::Draw,player:Some(YOU)});
    let waiting=AbilityRef {card:g.cards.id_of(&CREATURE),index:0};
    cantrip_api::Game::delayed_trigger(&mut cx(&mut g),DelayedWhen::Step {step:Step::Draw,on_turn_of:Some(YOU)},creature,YOU,waiting,Target::None);
    let hand=g.state.zone(Zone::hand(YOU)).len();begin(&mut g,YOU,Step::Draw);
    assert_eq!(g.state.delayed.len(),1);assert!(g.state.triggers.is_empty());
    assert_eq!(g.state.zone(Zone::hand(YOU)).len(),hand);assert!(!began(&g,Step::Draw));assert!(g.chars(creature).has(Keyword::Flying));assert_eq!(g.state.agenda.len(),1);
    let other=g.state.zone(Zone::hand(THEM)).len();begin(&mut g,THEM,Step::Draw);assert_eq!(g.state.zone(Zone::hand(THEM)).len(),other+1);
    cx(&mut g).add_effect(source,source,EffectKind::LoseAllAbilities,Duration::Indefinite);
    begin(&mut g,YOU,Step::Draw);assert_eq!(g.state.zone(Zone::hand(YOU)).len(),hand+1);assert!(!g.chars(creature).has(Keyword::Flying));assert!(g.state.delayed.is_empty());assert!(g.state.triggers.iter().any(|t|t.source==creature));
}
#[test]
fn copies_supply_skips_and_ability_loss_disables_them(){
    let mut g=setup();let source=put(&mut g,&LAND,YOU);let card=g.cards.id_of(&UPKEEP);cx(&mut g).set_copy(source,Copiable::of(card));
    begin(&mut g,YOU,Step::Upkeep);assert!(!began(&g,Step::Upkeep));
    cx(&mut g).add_effect(source,source,EffectKind::LoseAllAbilities,Duration::Indefinite);begin(&mut g,YOU,Step::Upkeep);assert!(began(&g,Step::Upkeep));
}
#[test]
fn the_starting_players_first_draw_step_is_entirely_skipped_in_two_player_games(){
    let mut g=setup();g.state.update_turn(|t|{t.number=1;t.starting=YOU;});let hand=g.state.zone(Zone::hand(YOU)).len();
    begin(&mut g,YOU,Step::Draw);assert_eq!(g.state.zone(Zone::hand(YOU)).len(),hand);assert!(!began(&g,Step::Draw));assert_eq!(g.state.agenda.len(),1);
    begin(&mut g,THEM,Step::Draw);assert!(began(&g,Step::Draw));
}

#[test]
#[should_panic(expected="skipped steps currently support Untap, Upkeep and Draw")]
fn unsupported_combat_step_skips_are_rejected_instead_of_breaking_damage_round_progression(){
    static BAD:CardDef=CardDef {name:"Unsupported skip fixture",skip_steps:&[SkipStep {applies:all,step:Step::CombatDamage}],..UNTAP};
    CardTable::new([&BAD]);
}

#[test]
fn an_additional_draw_step_in_the_first_turn_is_not_the_skipped_ordinary_draw(){
    let mut g=setup();
    g.state.update_turn(|t|{t.number=1;t.starting=YOU;t.phase_additional=true;});
    let hand=g.state.zone(Zone::hand(YOU)).len();
    begin(&mut g,YOU,Step::Draw);
    assert_eq!(g.state.zone(Zone::hand(YOU)).len(),hand+1);
    assert!(began(&g,Step::Draw));
}

#[test]
fn an_additional_untap_respects_skips_without_beginning_another_turn(){
    let mut g=setup();let source=put(&mut g,&UNTAP,YOU);let creature=put(&mut g,&PHASING,YOU);
    g.state.update_obj(creature,|o|o.tapped=true);
    g.state.update_turn(|t|t.phase_additional=true);
    g.state.update_player(YOU,|p|p.lands_played=1);
    cx(&mut g).add_effect(source,creature,EffectKind::AddKeywords(Keywords::of(&[Keyword::Flying])),Duration::UntilNextStepOf{step:Step::Untap,player:Some(YOU)});
    let turn_began=g.state.player(YOU).turn_began;
    begin(&mut g,YOU,Step::Untap);
    assert_eq!(g.state.player(YOU).turn_began,turn_began);
    assert_eq!(g.state.player(YOU).lands_played,1);
    assert!(g.state.obj(creature).tapped);
    assert!(g.state.obj(creature).marks.phased.is_none());
    assert!(g.chars(creature).has(Keyword::Flying));
}

#[test]
fn controller_relative_deadline_is_checked_before_phasing_in() {
    let mut g=setup();let creature=put(&mut g,&CREATURE,YOU);
    cx(&mut g).add_effect(creature,creature,EffectKind::AddKeywords(Keywords::of(&[Keyword::Flying])),Duration::UntilItsControllersNextUntapStep);
    crate::phasing::phase_out(&mut cx(&mut g),&[creature],None,false);
    begin(&mut g,YOU,Step::Untap);
    assert!(g.state.obj(creature).marks.phased.is_none());
    assert!(g.chars(creature).has(Keyword::Flying));
}
