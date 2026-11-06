//! Forced CR113.2c/602.5c/607.5 positions; definitions are test fixtures for the reported printed users.
use crate::{Cx, Game, Setup};
use cantrip_api::*;
use cantrip_state::{Frame, FrameKind};
const YOU: PlayerId = PlayerId(0);
fn untap(g: &mut dyn cantrip_api::Game, i: &StackItem) -> Result<(), Suspend> { g.untap(i.object); Ok(()) }
fn pump(g: &mut dyn cantrip_api::Game, i: &StackItem) -> Result<(), Suspend> { g.add_effect(i.object, i.object, EffectKind::Pump(1,0), Duration::EndOfTurn); Ok(()) }
fn bats_limit(q: &dyn Query, a: &Asker) -> bool {
    cantrip_mechanics::history::this_ability_activations_this_turn(q,a) < 2
}
fn priest(g: &mut dyn cantrip_api::Game, a: &Asker, _: Option<Color>) -> Result<(), Suspend> {
    g.add_mana(a.controller, Mana::W, 1);
    let card = g.card_id(&PRIEST);
    let n = cantrip_mechanics::history::this_ability_activations_this_turn(g,a);
    if n >= 4 { g.delayed_trigger(DelayedWhen::Step { step: Step::End, on_turn_of: None }, a.source, a.controller, AbilityRef::delayed(card,0), Target::None); }
    Ok(())
}
fn nothing(_: &mut dyn cantrip_api::Game, _: &StackItem) -> Result<(), Suspend> { Ok(()) }
static TOUCH: CardDef = CardDef { name: "Touch of Vitae", delayed: &[ability(Cost::NONE, untap, "{0}: Untap this creature. Activate only once.").once()], ..CardDef::BLANK };
static BODY: CardDef = CardDef { name: "Ornithopter", types: TypeSet::of(&[CardType::Artifact, CardType::Creature]), power: Some(0), toughness: Some(2), ..CardDef::BLANK };
static BATS: CardDef = CardDef { name: "Vampire Bats", types: TypeSet::of(&[CardType::Creature]), power: Some(0), toughness: Some(1), abilities: &[ability(cost("{B}"),pump,"{B}: This creature gets +1/+0 until end of turn. Activate no more than twice each turn.").condition(bats_limit)], ..CardDef::BLANK };
static PRIEST: CardDef = CardDef { name: "Farrelite Priest", types: TypeSet::of(&[CardType::Creature]), power: Some(1), toughness: Some(3), abilities: &[mana_ability(cost("{1}"), Adds { pool: ManaPool([1,0,0,0,0,0]), ..add_by(priest) }, "{1}: Add {W}. If this ability has been activated four or more times this turn, sacrifice this creature at the beginning of the next end step.")], delayed: &[delayed_trigger(nothing,"Sacrifice this creature.")], ..CardDef::BLANK };
fn itself(_: &dyn LayerQuery, s: ObjectId, id: ObjectId, _: &Chars) -> bool { s == id }
fn borrowed(q: &dyn LayerQuery, _: ObjectId, c: &mut Chars) {
    let cards: Vec<_> = [YOU,PlayerId(1)].into_iter().flat_map(|p| q.state().zone(Zone::graveyard(p)).iter().copied()).filter(|&id| q.chars(id).is(CardType::Creature)).map(|id| (id,q.chars(id).card)).collect();
    cantrip_mechanics::statics::grant_activated_abilities_from(q,&cards,c);
}
static OOZE: CardDef = CardDef { name: "Necrotic Ooze", types: TypeSet::of(&[CardType::Creature]), power: Some(4), toughness: Some(3), statics: &[static_ability(Layer::Ability,itself,borrowed,"This creature has all activated abilities of all creature cards in all graveyards.")], ..CardDef::BLANK };
static DANCE: CardDef = CardDef { name: "Dance of Many", types: TypeSet::of(&[CardType::Enchantment]), abilities: &[ability(Cost::NONE,nothing,"Create a token.")], ..CardDef::BLANK };
fn granted_limit(q:&dyn Query,_:ObjectId,ability:AbilityRef)->usize {
    q.state().zone(Zone::graveyard(YOU)).iter().filter(|&&id|q.obj(id).card==ability.card).count()
}
static LOCUS: CardDef = CardDef { name:"Locus of Enlightenment", types:TypeSet::of(&[CardType::Artifact]),
    statics:&[static_ability(Layer::Ability,itself,borrowed,"This permanent has each activated ability of the cards. You may activate each of those abilities only once each turn.")],
    granted_once_each_turn:Some(granted_limit),..CardDef::BLANK };
static ONCE: CardDef = CardDef { name:"Once creature",types:TypeSet::of(&[CardType::Creature]),power:Some(1),toughness:Some(1),
    abilities:&[ability(Cost::NONE,nothing,"{0}: Do nothing. Activate only once.").once()],..CardDef::BLANK };
static FREE: CardDef = CardDef { name:"Free creature",types:TypeSet::of(&[CardType::Creature]),power:Some(1),toughness:Some(1),
    abilities:&[ability(Cost::NONE,nothing,"{0}: Do nothing.")],..CardDef::BLANK };
static ONCE_TURN: CardDef = CardDef { name:"Once each turn creature",types:TypeSet::of(&[CardType::Creature]),power:Some(1),toughness:Some(1),
    abilities:&[ability(Cost::NONE,nothing,"{0}: Do nothing. Activate only once each turn.").once_each_turn()],..CardDef::BLANK };
fn sacrificial_mana(g:&mut dyn cantrip_api::Game,a:&Asker,_:Option<Color>)->Result<(),Suspend> {
    assert!(g.obj(a.source).became.is_some(),"the source left paying the cost");
    assert_ne!(a.instance.unwrap().acquisition.origin,ObjectId::NONE,"the physical donor survives in captured metadata");
    assert_eq!(cantrip_mechanics::history::this_ability_activations_this_turn(g,a),1);
    g.add_mana(a.controller,Mana::W,1);Ok(())
}
static SAC_MANA: CardDef = CardDef { name:"Sacrificial mana creature",types:TypeSet::of(&[CardType::Creature]),power:Some(1),toughness:Some(1),
    abilities:&[mana_ability(Cost {sacrifice_self:true,..Cost::NONE},Adds {pool:ManaPool([1,0,0,0,0,0]),..add_by(sacrificial_mana)},"Sacrifice this creature: Add {W}.")],..CardDef::BLANK };
static LENDER: CardDef = CardDef { name:"External static lender",delayed:&[granted_static(Layer::Ability,borrowed,"This creature has all activated abilities of creature cards in graveyards.")],..CardDef::BLANK };
fn cx(g: &mut Game, frame: usize) -> Cx<'_> { Cx { g, frame, cursor:0, mana_source:None, entering:None, lineage:vec![], damage: None, asked_at:None, doer:None, producing:None } }
fn game() -> Game {
    let cards = Box::leak(Box::new(CardTable::new([&TOUCH,&BODY,&BATS,&PRIEST,&OOZE,&DANCE,&LOCUS,&ONCE,&FREE,&ONCE_TURN,&SAC_MANA,&LENDER])));
    let mut g=Game::new(cards,&Setup { decks:vec![vec![],vec![]], ..Default::default() });
    while !g.state.agenda.is_empty() { g.state.frame_remove(0); }
    g.state.update_turn(|t| { t.number=3;t.active=YOU;t.priority=YOU;t.set_step(Step::PrecombatMain); });
    for p in [YOU,PlayerId(1)] { g.state.update_player(p,|s| {s.drew_from_empty=false;s.turn_began=1;}); }
    g
}
fn put(g: &mut Game, def: &'static CardDef, zone:Zone) -> ObjectId {
    let id=g.state.push_obj(Obj::new(g.cards.id_of(def),YOU,zone)); let stamp=g.state.timestamp();
    g.state.update_obj(id,|o| o.timestamp=stamp);g.state.zone_push(zone,id);g.state.see_statics();id
}
fn options(g:&mut Game,id:ObjectId)->Vec<ActionOption> { crate::actions::priority_options(&mut cx(g,usize::MAX),YOU).into_iter().filter(|o|o.source==Some(id)&&o.kind==ActionKind::ActivateAbility).collect() }
fn activate(g:&mut Game,id:ObjectId,index:u8) {
    while !g.state.agenda.is_empty() {g.state.frame_remove(0);}
    g.state.frame_push(Frame::new(FrameKind::Priority));g.run();
    let r=g.pending().cloned().expect("priority");
    let RequestKind::Priority {options}=r.kind else {panic!("priority, got {:?}",r.kind)};
    let option=options.iter().find(|o|o.source==Some(id)&&o.index==index&&o.kind==ActionKind::ActivateAbility)
        .unwrap_or_else(||panic!("independent ability index{index}, turn{}, pool{:?}, options{options:?}",g.state.turn.number,g.state.player(YOU).pool));
    let before=g.log.iter().filter(|r|matches!(r.event,Event::AbilityActivated {source,..} if source==id)).count();
    g.answer(r.id,r.player,Answer::Act(Action::of(option))).unwrap();
    for _ in 0..30 {
        g.run();
        let activated=g.log.iter().filter(|r|matches!(r.event,Event::AbilityActivated {source,..} if source==id)).count()>before;
        if activated&&g.state.stack.is_empty() {return;}
        let r=g.pending().cloned().expect("an outstanding question");
        let answer=match &r.kind {
            RequestKind::Priority {..}=>Answer::Pass,
            RequestKind::PayCosts {options,..}=>Answer::Act(Action::of(options.iter().find(|o|o.kind==ActionKind::ConfirmCasting)
                .or_else(||options.iter().find(|o|o.kind==ActionKind::AssignMana&&o.mana==Some(Mana::C)))
                .or_else(||options.iter().find(|o|o.kind==ActionKind::AssignMana))
                .unwrap_or_else(||panic!("the supplied pool pays: {:?}",r.kind)))),
            other=>panic!("unexpected {other:?}"),
        };
        g.answer(r.id,r.player,answer).unwrap();
    }
    panic!("activation did not finish");
}
fn grant(g:&mut Game,source:ObjectId,id:ObjectId)->u32 {let stamp=g.state.timestamp();g.state.effect_push(ContinuousEffect {source,controller:YOU,timestamp:stamp,affected:id,kind:EffectKind::AddAbility(AbilityRef::delayed(g.cards.id_of(&TOUCH),0)),duration:Duration::EndOfTurn,words:Words::NONE});stamp}
#[test]
fn acquisition_same_live_once_grant_stays_spent() {
    let mut g=game();let s=put(&mut g,&TOUCH,Zone::graveyard(YOU));let id=put(&mut g,&BODY,Zone::BATTLEFIELD);grant(&mut g,s,id);
    assert_eq!(options(&mut g,id).len(),1);activate(&mut g,id,0);assert!(options(&mut g,id).is_empty());
}
#[test]
fn acquisition_touch_fresh_grant_after_expiry_can_be_activated() {
    let mut g=game();let s=put(&mut g,&TOUCH,Zone::graveyard(YOU));let id=put(&mut g,&BODY,Zone::BATTLEFIELD);let old=grant(&mut g,s,id);activate(&mut g,id,0);
    g.state.effects_retain(|e|e.timestamp!=old);g.state.update_turn(|t|t.number=5);grant(&mut g,s,id);
    assert_eq!(options(&mut g,id).len(),1,"a new grant is not the once-only ability already activated");
}
#[test]
fn acquisition_bats_remaining_donor_keeps_its_own_two_uses() {
    let mut g=game();let id=put(&mut g,&OOZE,Zone::BATTLEFIELD);let old=put(&mut g,&BATS,Zone::graveyard(YOU));put(&mut g,&BATS,Zone::graveyard(YOU));
    g.state.add_mana(YOU,Mana::B,8,id,None,ManaLasts::Step,false);assert_eq!(options(&mut g,id).len(),2);activate(&mut g,id,0);activate(&mut g,id,0);
    cantrip_api::Game::exile_all(&mut cx(&mut g,usize::MAX),&[old]);
    assert_eq!(options(&mut g,id).len(),1,"the other donor's ability has not been activated");
}
#[test]
fn acquisition_priest_three_uses_of_one_and_one_of_another_is_not_four_of_either() {
    let mut g=game();let id=put(&mut g,&OOZE,Zone::BATTLEFIELD);put(&mut g,&PRIEST,Zone::graveyard(YOU));put(&mut g,&PRIEST,Zone::graveyard(YOU));
    g.state.add_mana(YOU,Mana::C,8,id,None,ManaLasts::Step,false);assert_eq!(options(&mut g,id).len(),2);for index in [0,0,0,1] {activate(&mut g,id,index);}
    assert_eq!(g.state.player(YOU).pool.get(Mana::W),4);assert!(g.state.delayed.is_empty(),"neither independent instance reached four activations");
}
#[test]
fn acquisition_priest_four_uses_of_one_donor_schedules_sacrifice() {
    let mut g=game();let id=put(&mut g,&OOZE,Zone::BATTLEFIELD);put(&mut g,&PRIEST,Zone::graveyard(YOU));
    g.state.add_mana(YOU,Mana::C,8,id,None,ManaLasts::Step,false);for _ in 0..4 {activate(&mut g,id,0);}
    assert_eq!(g.state.player(YOU).pool.get(Mana::W),4);assert_eq!(g.state.delayed.len(),1);
}
#[test]
fn acquisition_single_bats_donor_has_exactly_two_uses() {
    let mut g=game();let id=put(&mut g,&OOZE,Zone::BATTLEFIELD);put(&mut g,&BATS,Zone::graveyard(YOU));
    g.state.add_mana(YOU,Mana::B,8,id,None,ManaLasts::Step,false);activate(&mut g,id,0);activate(&mut g,id,0);
    assert!(options(&mut g,id).is_empty());
}
#[test]
fn acquisition_dance_token_history_distinguishes_another_copy_acquisition() {
    let mut g=game();let id=put(&mut g,&DANCE,Zone::BATTLEFIELD);let card=g.cards.id_of(&DANCE);let token=g.cards.id_of(&BODY);let mut causes=vec![];
    for acquired in [Acquired::OWN,Acquired {by:AcquiredBy::Copy,stamp:100,part:0}] {
        let mut item=crate::actions::stack_item(StackKind::Triggered,id,AbilityRef {card,index:0},YOU,&[]);item.acquired=acquired;g.state.stack_push(item);let serial=g.state.stack.last().unwrap().serial;
        let at=g.state.agenda.len();g.state.frame_push(Frame::new(FrameKind::Resolve {serial}));let made=cantrip_api::Game::create_tokens(&mut cx(&mut g,at),token,YOU,1,false).unwrap()[0];
        causes.push(g.log.iter().rev().find(|r|matches!(r.event,Event::Entered {permanent,..} if permanent==made)).unwrap().cause);
        g.state.frame_remove(at);g.state.stack_remove(g.state.stack.len()-1);
    }
    assert_ne!(causes[0],causes[1],"the later linked ability acquisition cannot claim the original token's creation");
}

#[test]
fn acquisition_concurrent_grants_each_have_a_once_budget_and_keep_it_after_slot_shift() {
    let mut g=game();let s=put(&mut g,&TOUCH,Zone::graveyard(YOU));let id=put(&mut g,&BODY,Zone::BATTLEFIELD);
    let first=grant(&mut g,s,id);grant(&mut g,s,id);assert_eq!(options(&mut g,id).len(),2);
    activate(&mut g,id,1);assert_eq!(options(&mut g,id).iter().map(|o|o.index).collect::<Vec<_>>(),vec![0]);
    g.state.effects_retain(|e|e.timestamp!=first);
    assert!(options(&mut g,id).is_empty(),"packing slots must not replenish the remaining grant");
    grant(&mut g,s,id);assert_eq!(options(&mut g,id).iter().map(|o|o.index).collect::<Vec<_>>(),vec![1],"a same-turn new grant is fresh");
}

#[test]
fn acquisition_both_bats_donors_have_two_uses_without_pooling() {
    let mut g=game();let id=put(&mut g,&OOZE,Zone::BATTLEFIELD);put(&mut g,&BATS,Zone::graveyard(YOU));put(&mut g,&BATS,Zone::graveyard(YOU));
    g.state.add_mana(YOU,Mana::B,8,id,None,ManaLasts::Step,false);
    activate(&mut g,id,0);activate(&mut g,id,0);
    assert_eq!(options(&mut g,id).iter().map(|o|o.index).collect::<Vec<_>>(),vec![1]);
    activate(&mut g,id,1);activate(&mut g,id,1);assert!(options(&mut g,id).is_empty());
}

#[test]
fn acquisition_limited_static_instances_do_not_share_with_an_unlimited_resolved_grant() {
    let mut g=game();let id=put(&mut g,&LOCUS,Zone::BATTLEFIELD);put(&mut g,&FREE,Zone::graveyard(YOU));put(&mut g,&FREE,Zone::graveyard(YOU));
    let ability=AbilityRef {card:g.cards.id_of(&FREE),index:0};
    cantrip_api::Game::add_effect(&mut cx(&mut g,usize::MAX),id,id,EffectKind::AddAbility(ability),Duration::EndOfTurn);
    assert_eq!(options(&mut g,id).len(),3);activate(&mut g,id,0);
    assert_eq!(options(&mut g,id).iter().map(|o|o.index).collect::<Vec<_>>(),vec![1,2]);
    activate(&mut g,id,2);activate(&mut g,id,2);activate(&mut g,id,1);
    assert_eq!(options(&mut g,id).iter().map(|o|o.index).collect::<Vec<_>>(),vec![2]);
}

#[test]
fn acquisition_own_text_copy_and_new_copy_have_independent_once_budgets() {
    let mut g=game();let id=put(&mut g,&ONCE,Zone::BATTLEFIELD);let donor=put(&mut g,&ONCE,Zone::BATTLEFIELD);
    activate(&mut g,id,0);assert!(options(&mut g,id).is_empty());
    let values=cx(&mut g,usize::MAX).copiable_values(donor);
    cantrip_api::Game::become_copy(&mut cx(&mut g,usize::MAX),donor,id,values,Duration::EndOfTurn);
    assert_eq!(options(&mut g,id).len(),1);activate(&mut g,id,0);assert!(options(&mut g,id).is_empty());
    cantrip_api::Game::become_copy(&mut cx(&mut g,usize::MAX),donor,id,values,Duration::EndOfTurn);
    assert_eq!(options(&mut g,id).len(),1);
}

#[test]
fn acquisition_a_fresh_copy_of_static_granting_text_acquires_a_new_instance() {
    let mut g=game();let id=put(&mut g,&OOZE,Zone::BATTLEFIELD);let source=put(&mut g,&OOZE,Zone::BATTLEFIELD);put(&mut g,&ONCE,Zone::graveyard(YOU));
    activate(&mut g,id,0);assert!(options(&mut g,id).is_empty());
    let values=cx(&mut g,usize::MAX).copiable_values(source);
    cantrip_api::Game::become_copy(&mut cx(&mut g,usize::MAX),source,id,values,Duration::EndOfTurn);
    assert_eq!(options(&mut g,id).len(),1,"the copied static ability is a new acquisition of the borrowed ability");
}

#[test]
fn acquisition_an_external_layer_six_static_grant_surviving_a_copy_keeps_its_budget() {
    let mut g=game();let id=put(&mut g,&BODY,Zone::BATTLEFIELD);let source=put(&mut g,&OOZE,Zone::BATTLEFIELD);let lender=put(&mut g,&LENDER,Zone::graveyard(YOU));put(&mut g,&ONCE,Zone::graveyard(YOU));
    let ability=AbilityRef::delayed(g.cards.id_of(&LENDER),0);
    cantrip_api::Game::add_effect(&mut cx(&mut g,usize::MAX),lender,id,EffectKind::AddAbility(ability),Duration::EndOfTurn);
    let available=options(&mut g,id);assert_eq!(available.len(),1);activate(&mut g,id,available[0].index);
    let key=g.log.iter().find_map(|r|if let Event::AbilityActivated {instance,..}=r.event {Some(instance)} else {None}).unwrap();
    let values=cx(&mut g,usize::MAX).copiable_values(source);cantrip_api::Game::become_copy(&mut cx(&mut g,usize::MAX),source,id,values,Duration::EndOfTurn);
    assert_eq!(options(&mut g,id).len(),1,"only the newly copied text's grant is fresh; the external static survives unchanged");
    let chars=crate::layers::chars(g.cards,&g.state,&g.log,id);
    assert!(chars.abilities.iter().enumerate().any(|(slot,a)|a.is_some()&&crate::layers::instance(g.cards,&g.state,&g.log,id,&chars,slot)==key));
}

#[test]
fn acquisition_control_changes_do_not_reset_a_live_grant() {
    let mut g=game();let s=put(&mut g,&TOUCH,Zone::graveyard(YOU));let id=put(&mut g,&BODY,Zone::BATTLEFIELD);grant(&mut g,s,id);activate(&mut g,id,0);
    g.state.update_obj(id,|o|o.controller=PlayerId(1));g.state.update_obj(id,|o|o.controller=YOU);
    assert!(options(&mut g,id).is_empty());
}

#[test]
fn acquisition_self_granted_own_text_keeps_link_group_but_gets_its_own_budget() {
    let mut g=game();let id=put(&mut g,&ONCE,Zone::BATTLEFIELD);let ability=AbilityRef {card:g.cards.id_of(&ONCE),index:0};
    activate(&mut g,id,0);
    cantrip_api::Game::add_effect(&mut cx(&mut g,usize::MAX),id,id,EffectKind::AddAbility(ability),Duration::EndOfTurn);
    let chars=crate::layers::chars(g.cards,&g.state,&g.log,id);
    assert_eq!(crate::layers::acquired(g.cards,&g.state,&g.log,id,&chars,1),Acquired::OWN,"own text's linked abilities stay linked");
    assert_ne!(crate::layers::instance(g.cards,&g.state,&g.log,id,&chars,0),crate::layers::instance(g.cards,&g.state,&g.log,id,&chars,1));
    assert_eq!(options(&mut g,id).iter().map(|o|o.index).collect::<Vec<_>>(),vec![1]);
    activate(&mut g,id,1);assert!(options(&mut g,id).is_empty());
}

#[test]
fn acquisition_fork_and_tree_undo_redo_preserve_activation_metadata() {
    let mut original=game();let id=put(&mut original,&OOZE,Zone::BATTLEFIELD);put(&mut original,&BATS,Zone::graveyard(YOU));put(&mut original,&BATS,Zone::graveyard(YOU));
    original.state.add_mana(YOU,Mana::B,8,id,None,ManaLasts::Step,false);
    original.state.frame_push(Frame::new(FrameKind::Priority));original.run();
    let mut fork=original.fork_for_search();let root=fork.checkpoint();let before=fork.state.clone();let before_log=fork.log.clone();
    activate(&mut fork,id,1);let after=fork.checkpoint();let state=fork.state.clone();let log=fork.log.clone();
    assert_ne!(log,before_log);assert_eq!(original.log,before_log);
    fork.goto(root);assert!(fork.state==before);assert_eq!(fork.log,before_log);
    fork.goto(after);assert!(fork.state==state);assert_eq!(fork.log,log);
    assert_eq!(options(&mut fork,id).len(),2,"one use leaves this donor one and the other donor two");
}

#[test]
fn acquisition_once_each_turn_uses_identity_instead_of_packed_slots() {
    let mut g=game();let id=put(&mut g,&OOZE,Zone::BATTLEFIELD);let old=put(&mut g,&ONCE_TURN,Zone::graveyard(YOU));put(&mut g,&ONCE_TURN,Zone::graveyard(YOU));
    activate(&mut g,id,0);assert_eq!(options(&mut g,id).iter().map(|o|o.index).collect::<Vec<_>>(),vec![1]);
    cantrip_api::Game::exile_all(&mut cx(&mut g,usize::MAX),&[old]);
    assert_eq!(options(&mut g,id).len(),1);activate(&mut g,id,0);assert!(options(&mut g,id).is_empty());
    g.state.update_turn(|t|t.number+=2);assert_eq!(options(&mut g,id).len(),1);
}

#[test]
fn acquisition_mana_callback_keeps_identity_captured_before_source_sacrifice() {
    let mut g=game();let id=put(&mut g,&OOZE,Zone::BATTLEFIELD);put(&mut g,&SAC_MANA,Zone::graveyard(YOU));
    activate(&mut g,id,0);assert_eq!(g.state.player(YOU).pool.get(Mana::W),1);
}

#[test]
fn acquisition_source_returning_to_battlefield_is_a_fresh_incarnation() {
    let mut g=game();let id=put(&mut g,&ONCE,Zone::BATTLEFIELD);activate(&mut g,id,0);assert!(options(&mut g,id).is_empty());
    let exile=cantrip_api::Game::exile_all(&mut cx(&mut g,usize::MAX),&[id])[0];
    let returned=cantrip_api::Game::put_onto_battlefield(&mut cx(&mut g,usize::MAX),exile,YOU,false).unwrap();
    assert_ne!(id,returned);assert_eq!(options(&mut g,returned).len(),1);
}

#[test]
fn acquisition_stack_copy_retains_key_without_another_activation() {
    let mut g=game();let s=put(&mut g,&TOUCH,Zone::graveyard(YOU));let id=put(&mut g,&BODY,Zone::BATTLEFIELD);grant(&mut g,s,id);
    g.state.frame_push(Frame::new(FrameKind::Priority));g.run();
    let r=g.pending().unwrap().clone();let RequestKind::Priority {options}=r.kind else {panic!("priority")};
    let option=options.iter().find(|o|o.source==Some(id)&&o.kind==ActionKind::ActivateAbility).unwrap();
    g.answer(r.id,r.player,Answer::Act(Action::of(option))).unwrap();
    for _ in 0..5 {
        g.run();if g.log.iter().any(|r|matches!(r.event,Event::AbilityActivated {source,..} if source==id)) {break;}
        let r=g.pending().unwrap().clone();let RequestKind::PayCosts {options,..}=r.kind else {panic!("pay costs")};
        let option=options.iter().find(|o|o.kind==ActionKind::ConfirmCasting).unwrap();g.answer(r.id,r.player,Answer::Act(Action::of(option))).unwrap();
    }
    assert_eq!(g.state.stack.len(),1);let item=g.state.stack[0];let log=g.log.clone();
    let copied=cantrip_api::Game::copy_ability(&mut cx(&mut g,usize::MAX),Target::Ability(item.serial),YOU,false).unwrap().unwrap();
    assert_ne!(item.serial,copied);assert_eq!(g.state.stack.last().unwrap().instance,item.instance);
    assert_eq!(g.state.stack.last().unwrap().acquired,item.acquired);
    assert_eq!(&g.log[..log.len()], &log);
    assert!(matches!(g.log[log.len()..], [record] if matches!(record.event, Event::AbilityCopied { copy, original } if copy == copied && original == item.serial)), "the only new record is the copy lineage, not another activation");
}

#[test]
fn acquisition_loop_position_canonicalizes_donor_ids_and_grant_timestamps() {
    let mut g=game();let id=put(&mut g,&OOZE,Zone::BATTLEFIELD);let donor=put(&mut g,&BATS,Zone::graveyard(YOU));let other=put(&mut g,&BATS,Zone::graveyard(YOU));
    let mut item=crate::actions::stack_item(StackKind::Activated,id,AbilityRef {card:g.cards.id_of(&BATS),index:0},YOU,&[]);
    item.instance=AbilityAcquisition {acquired:Acquired {by:AcquiredBy::Grant,stamp:100,part:0},origin:donor,ordinal:0,static_grant:true};g.state.stack_push(item);
    let before=crate::loops::position(&g.state);
    let mut same=g.state.clone();
    for object in [id,donor,other] {same.update_obj(object,|o|o.timestamp+=1000);}
    let mut changed=same.stack[0];changed.instance.acquired.stamp+=1000;same.stack_set(0,changed);
    assert_eq!(crate::loops::position(&same),before,"absolute acquisition timestamps are not loop identity");
    let mut changed=same.stack[0];changed.instance.origin=other;same.stack_set(0,changed);
    assert_ne!(crate::loops::position(&same),before,"different live donors remain distinct");
}

#[test]
fn acquisition_metadata_sizes_are_explicit() {
    println!("Event={} Cause={} Record={} StackItem={} PendingTrigger={} Asker={} Chars={} Acquisition={}",
        std::mem::size_of::<Event>(),std::mem::size_of::<Cause>(),std::mem::size_of::<Record>(),std::mem::size_of::<StackItem>(),
        std::mem::size_of::<cantrip_state::PendingTrigger>(),std::mem::size_of::<Asker>(),std::mem::size_of::<Chars>(),std::mem::size_of::<AbilityAcquisition>());
}

#[test]
fn paid_cost_layout_baseline() {
    println!("Event={} Record={} Cause={} StackItem={} ManaInterval={} Cost={}", std::mem::size_of::<Event>(), std::mem::size_of::<Record>(), std::mem::size_of::<Cause>(), std::mem::size_of::<StackItem>(), std::mem::size_of::<crate::ManaInterval>(), std::mem::size_of::<Cost>());
}
