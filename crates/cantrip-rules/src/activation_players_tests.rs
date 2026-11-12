//! CR 602.2, 113.8, 605.3 and 302.6: explicit activators change permission, not the source's controller or costs.
//! Printed users: Armageddon Clock, Clergy of the Holy Nimbus, Personal Incarnation and Land's Edge.
use super::*;
use crate::Game as RulesGame;
use cantrip_state::{Frame, FrameKind};
use std::sync::OnceLock;
const YOU: PlayerId = PlayerId(0);
const THEM: PlayerId = PlayerId(1);
fn nothing(_: &mut dyn Game, _: &StackItem) -> Result<(), Suspend> { Ok(()) }
fn reward(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> { g.gain_life(item.controller, 3); Ok(()) }
fn opponent(q: &dyn Query, a: &Asker, player: PlayerId) -> bool {
    assert_eq!(a.controller, q.state().obj(a.source).controller, "permission is asked as the source");
    a.controller != player
}
fn owner(q: &dyn Query, a: &Asker, player: PlayerId) -> bool { q.state().obj(a.source).owner == player }
fn actor_is_opponent(q: &dyn Query, a: &Asker) -> bool { a.controller != q.state().obj(a.source).controller }
static SHARED: CardDef = CardDef {
    name: "Shared activation probe", types: TypeSet::of(&[CardType::Artifact]),
    abilities: &[
        ability(Cost { life: 2, ..Cost::NONE }, reward, "Pay 2 life: Gain 3 life. Any player may activate this ability.").any_player_may_activate().once_each_turn(),
        ability(Cost::NONE, nothing, "{0}: Nothing."),
        ability(Cost::NONE, nothing, "{0}: Nothing. Only opponents may activate.").activated_by(opponent).condition(actor_is_opponent),
        ability(Cost::NONE, nothing, "{0}: Nothing. Only its owner may activate.").activated_by(owner),
    ], ..CardDef::BLANK
};
static MANA: CardDef = CardDef {
    name: "Shared mana probe", abilities: &[
        mana_ability(TAP, add(Mana::G), "{T}: Add {G}. Any player may activate.").any_player_may_activate(),
        mana_ability(TAP, add(Mana::C), "{T}: Add {C}."),
    ], ..SHARED
};
static PAID: CardDef = CardDef {
    name: "Shared mana payment probe", abilities: &[ability(cost("{G}"), nothing, "{G}: Nothing.")], ..SHARED
};
static SACRIFICE: CardDef = CardDef {
    name: "Shared sacrifice probe", abilities: &[ability(SACRIFICE_THIS, nothing, "Sacrifice this: Nothing. Any player may activate.").any_player_may_activate()], ..SHARED
};
static HAND: CardDef = CardDef {
    name: "Shared hidden-zone control", abilities: &[ability(Cost::NONE, nothing, "Nothing from hand").from(ZoneKind::Hand).any_player_may_activate()], ..SHARED
};
static CREATURE: CardDef = CardDef { name: "Shared creature probe", types: TypeSet::of(&[CardType::Creature]), power: Some(1), toughness: Some(1), ..MANA };
static NORMAL: CardDef = CardDef { name: "Ordinary activation probe", abilities: &[ability(Cost::NONE, nothing, "{0}: Nothing.")], ..SHARED };
static SORCERY: CardDef = CardDef { name: "Shared timing probe", abilities: &[ability(Cost::NONE, nothing, "{0}: Nothing. Activate only as a sorcery.").any_player_may_activate().sorcery_speed()], ..SHARED };
static RETURN: CardDef = CardDef { name: "Shared return cost probe", abilities: &[ability(RETURN_THIS,nothing,"Return this: Nothing. Any player may activate.").any_player_may_activate()],..SHARED };
static EXERT: CardDef = CardDef { name: "Shared exert cost probe", abilities: &[ability(Cost {exert:true,..Cost::NONE},nothing,"Exert this: Nothing. Any player may activate.").any_player_may_activate()],..SHARED };
static UNATTACH: CardDef = CardDef { name: "Shared unattach cost probe", subtypes:SubtypeSet::of(&[Subtype::Equipment]), abilities: &[ability(Cost {unattach_self:true,..Cost::NONE},nothing,"Unattach this: Nothing. Any player may activate.").any_player_may_activate()],..SHARED };
fn board() -> RulesGame {
    static TABLE: OnceLock<CardTable> = OnceLock::new();
    let cards = TABLE.get_or_init(|| CardTable::new([&SHARED,&MANA,&PAID,&SACRIFICE,&HAND,&CREATURE,&NORMAL,&SORCERY,&RETURN,&EXERT,&UNATTACH]));
    let deck = vec![cards.id_of(&NORMAL); 20];
    let mut g = RulesGame::new(cards, &crate::Setup { decks: vec![deck;2], ..Default::default() });
    while !g.state.agenda.is_empty() { g.state.frame_remove(0); }
    g.state.frame_push(Frame::new(FrameKind::Priority));
    g.state.update_turn(|t| { t.active=THEM; t.priority=THEM; t.number=1; t.set_step(Step::PrecombatMain); });
    g.state.update_player(YOU, |p|p.turn_began=1);
    g.state.update_player(THEM, |p|p.turn_began=1);
    g
}
fn put(g:&mut RulesGame, def:&'static CardDef, controller:PlayerId, zone:Zone)->ObjectId {
    let id=g.state.push_obj(Obj::new(g.cards.id_of(def),controller,zone));
    g.state.zone_push(zone,id);g.state.see_statics();id
}
fn cx(g:&mut RulesGame)->Cx<'_> { Cx {g,frame:0,cursor:0,mana_source:None,entering:None,lineage:Vec::new(),damage:None,asked_at:None,doer:Some(THEM),producing:None} }
fn options(g:&mut RulesGame, player:PlayerId, source:ObjectId)->Vec<u8> {
    priority_options(&mut cx(g),player).into_iter().filter(|o|o.source==Some(source)).map(|o|o.index).collect()
}
#[test]
fn permissions_are_per_ability_and_relative_to_the_sources_current_controller_and_owner() {
    let mut g=board();let source=put(&mut g,&SHARED,YOU,Zone::BATTLEFIELD);
    assert_eq!(options(&mut g,YOU,source),[0,1,3]);
    assert_eq!(options(&mut g,THEM,source),[0,2]);
    cx(&mut g).add_effect(source,source,EffectKind::Control(THEM),Duration::EndOfTurn);
    ops::settle_lasting_and_control(&mut cx(&mut g));
    assert_eq!(options(&mut g,YOU,source),[0,2,3]);
    assert_eq!(options(&mut g,THEM,source),[0,1]);
    let a=AbilityRef {card:g.cards.id_of(&SHARED),index:2};
    assert!(activate(&mut cx(&mut g),THEM,source,a,2,&[],None,None,None).is_err(),"execution rechecks an expired permission");
}
#[test]
fn the_activator_pays_controls_the_stack_ability_and_uses_the_same_once_per_turn_limit() {
    let mut g=board();let source=put(&mut g,&SHARED,YOU,Zone::BATTLEFIELD);
    let a=AbilityRef {card:g.cards.id_of(&SHARED),index:0};
    activate(&mut cx(&mut g),THEM,source,a,0,&[],None,None,None).unwrap();
    assert_eq!(g.state.obj(source).controller,YOU);
    assert_eq!(g.state.player(YOU).life,20);assert_eq!(g.state.player(THEM).life,18);
    let item=*g.state.stack.last().unwrap();assert_eq!(item.controller,THEM);
    assert!(!options(&mut g,YOU,source).contains(&0));assert!(!options(&mut g,THEM,source).contains(&0));
    (activated(&cx(&mut g),a).unwrap().effect)(&mut cx(&mut g),&item).unwrap();
    assert_eq!(g.state.player(THEM).life,21);assert_eq!(g.state.player(YOU).life,20);
    assert!(g.log.iter().any(|r| matches!(r.event,Event::AbilityActivated {source:s,player:THEM,..} if s==source)));
}
#[test]
fn mana_permission_applies_to_payment_advice_and_immediate_resolution_for_the_activator() {
    let mut g=board();let source=put(&mut g,&MANA,YOU,Zone::BATTLEFIELD);
    let paid=put(&mut g,&PAID,THEM,Zone::BATTLEFIELD);
    assert_eq!(options(&mut g,THEM,source),[0]);assert_eq!(options(&mut g,YOU,source),[0,1]);
    assert_eq!(available(&cx(&mut g),THEM).most_unrestricted(),1);
    assert!(options(&mut g,THEM,paid).contains(&0),"the opposing source can pay an actual offered ability");
    assert!(mana_options_while_paying(&cx(&mut g),THEM,paid,Paying::Other).iter().any(|o|o.source==Some(source)));
    assert!(activate_mana(&mut cx(&mut g),THEM,source,1,None,false,&[]).is_err(),"the adjacent private ability stays private");
    activate_mana(&mut cx(&mut g),THEM,source,0,None,false,&[]).unwrap();
    assert!(g.state.obj(source).tapped);assert_eq!(g.state.obj(source).controller,YOU);
    assert!(g.state.stack.is_empty());assert_eq!(g.state.player(THEM).pool.get(Mana::G),1);assert_eq!(g.state.player(YOU).pool.get(Mana::G),0);
    assert!(g.log.iter().any(|r|matches!(r.event,Event::TappedForMana {permanent:s,player:THEM,..} if s==source)));
}
#[test]
fn granting_activation_permission_does_not_authorize_sacrifice_or_other_players_hidden_cards() {
    let mut g=board();let source=put(&mut g,&SACRIFICE,YOU,Zone::BATTLEFIELD);
    assert_eq!(options(&mut g,YOU,source),[0]);assert!(options(&mut g,THEM,source).is_empty());
    let hidden=put(&mut g,&HAND,YOU,Zone::hand(YOU));
    assert_eq!(options(&mut g,YOU,hidden),[0]);assert!(options(&mut g,THEM,hidden).is_empty());
}
#[test]
fn shared_sources_are_not_opponents_convoke_or_improvise_payers() {
    let mut g=board();let source=put(&mut g,&SHARED,YOU,Zone::BATTLEFIELD);
    let advice=available_as(&cx(&mut g),THEM,None,ManaWindow::Payment,false,Some(TapFor::spell(PayWith {improvise:true,..PayWith::NONE})));
    assert!(!advice.covers(&mana("{1}")),"an opposing artifact is not an improvise payment");
    assert!(!g.state.obj(source).tapped);
}
#[test]
fn copies_grants_ability_loss_and_phasing_use_the_current_ability_permission() {
    let mut g=board();let source=put(&mut g,&NORMAL,YOU,Zone::BATTLEFIELD);
    assert!(options(&mut g,THEM,source).is_empty());
    let shared=g.cards.id_of(&SHARED);cx(&mut g).set_copy(source,Copiable::of(shared));
    assert_eq!(options(&mut g,THEM,source),[0,2]);
    cx(&mut g).add_effect(source,source,EffectKind::LoseAllAbilities,Duration::EndOfTurn);
    assert!(options(&mut g,THEM,source).is_empty());
    g.state.effects_retain(|_|false);
    crate::phasing::phase_out(&mut cx(&mut g),&[source],None,false);
    assert!(options(&mut g,THEM,source).is_empty());
    let recipient=put(&mut g,&NORMAL,YOU,Zone::BATTLEFIELD);
    let grant=AbilityRef {card:shared,index:2};cx(&mut g).add_effect(source,recipient,EffectKind::AddAbility(grant),Duration::EndOfTurn);
    assert_eq!(options(&mut g,THEM,recipient),[1],"a granted ability retains its activation permission and actor condition");
}
#[test]
fn a_shared_creatures_tap_symbol_still_uses_its_own_controllers_summoning_sickness() {
    let mut g=board();let source=put(&mut g,&CREATURE,YOU,Zone::BATTLEFIELD);
    g.state.update_obj(source,|o|o.controlled_since=2);
    assert!(options(&mut g,THEM,source).is_empty());
    g.state.update_player(THEM,|p|p.turn_began=10);
    assert!(options(&mut g,THEM,source).is_empty(),"the activator's turn does not cure the source's sickness");
    g.state.update_player(YOU,|p|p.turn_began=3);
    assert_eq!(options(&mut g,THEM,source),[0]);
}
#[test]
fn permission_does_not_lift_sorcery_timing() {
    let mut g=board();let source=put(&mut g,&SORCERY,YOU,Zone::BATTLEFIELD);
    assert_eq!(options(&mut g,THEM,source),[0]);assert!(options(&mut g,YOU,source).is_empty());
    g.state.update_turn(|t|t.set_step(Step::DeclareAttackers));
    assert!(options(&mut g,THEM,source).is_empty());
}

#[test]
fn explicit_self_costs_use_the_permitted_source_and_exert_uses_the_payers_next_untap() {
    for def in [&RETURN,&EXERT,&UNATTACH] {
        let mut g=board();let source=put(&mut g,def,YOU,Zone::BATTLEFIELD);
        if std::ptr::eq(def,&UNATTACH) {
            let wearer=put(&mut g,&CREATURE,YOU,Zone::BATTLEFIELD);
            g.state.update_obj(source,|o|o.attached_to=Some(wearer));
        }
        assert_eq!(options(&mut g,THEM,source),[0]);
        let a=AbilityRef {card:g.cards.id_of(def),index:0};
        activate(&mut cx(&mut g),THEM,source,a,0,&[],None,None,None).unwrap();
        assert_eq!(g.state.stack.last().unwrap().controller,THEM);
        if std::ptr::eq(def,&RETURN) {
            let moved=g.state.obj(source).became.expect("returned source");
            assert_eq!(g.state.obj(moved).zone,Zone::hand(YOU));
        } else if std::ptr::eq(def,&EXERT) {
            assert_eq!(g.state.obj(source).marks.exerted_by,Some(THEM));
        } else { assert!(g.state.obj(source).attached_to.is_none()); }
    }
}
