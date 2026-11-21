//! CR 119.4, 614.1a: a replacement of life being paid ("If you would pay life, ...") sees a cost's or a resolving payment's
//! life as one (`LifeChange::paid`), and no other loss. Ashiok, Wicked Manipulator prints the replacement.
use super::*;
use crate::Game as RulesGame;
use cantrip_state::{Frame, FrameKind};
use std::sync::OnceLock;
const YOU: PlayerId = PlayerId(0);
const THEM: PlayerId = PlayerId(1);
fn nothing(_: &mut dyn Game, _: &StackItem) -> Result<(), Suspend> { Ok(()) }
fn you_would_pay_life(_: &dyn Query, asker: &Asker, proposal: &Proposal) -> bool {
    matches!(proposal, Proposal::LifeChange(l) if l.player == asker.controller && l.paid && l.amount < 0)
}
fn leave_it_unpaid(_: &dyn Query, _: &Asker, proposal: &mut Proposal) {
    if let Proposal::LifeChange(l) = proposal { l.amount = 0; }
}
/// What the replacement does in place of the payment: a gain of 1, so the tests see it applied.
fn gain_one(g: &mut dyn Game, asker: &Asker, before: &Proposal, after: &Proposal) {
    assert!(matches!(before, Proposal::LifeChange(l) if l.paid && l.amount < 0), "it is told the payment as proposed");
    assert!(matches!(after, Proposal::LifeChange(l) if l.paid && l.amount == 0), "and as it left it, still a payment");
    g.gain_life(asker.controller, 1);
}
static PAYMENT_REPLACER: CardDef = CardDef {
    name: "Pay life replacer", types: TypeSet::of(&[CardType::Enchantment]),
    replacements: &[replacement(ReplacementKind::LifeChange, you_would_pay_life, leave_it_unpaid, "If you would pay life, gain 1 life instead.").then(gain_one)],
    ..CardDef::BLANK
};
static PAYER: CardDef = CardDef {
    name: "Pay life probe", types: TypeSet::of(&[CardType::Artifact]),
    abilities: &[ability(Cost { life: 2, ..Cost::NONE }, nothing, "Pay 2 life: Nothing.")],
    ..CardDef::BLANK
};
fn board() -> RulesGame {
    static TABLE: OnceLock<CardTable> = OnceLock::new();
    let cards = TABLE.get_or_init(|| CardTable::new([&PAYMENT_REPLACER, &PAYER]));
    let deck = vec![cards.id_of(&PAYER); 20];
    let mut g = RulesGame::new(cards, &crate::Setup { decks: vec![deck; 2], ..Default::default() });
    while !g.state.agenda.is_empty() { g.state.frame_remove(0); }
    g.state.frame_push(Frame::new(FrameKind::Priority));
    g.state.update_turn(|t| { t.active = YOU; t.priority = YOU; t.number = 1; t.set_step(Step::PrecombatMain); });
    g.state.update_player(YOU, |p| p.turn_began = 1);
    g.state.update_player(THEM, |p| p.turn_began = 1);
    g
}
fn put(g: &mut RulesGame, def: &'static CardDef, controller: PlayerId, zone: Zone) -> ObjectId {
    let id = g.state.push_obj(Obj::new(g.cards.id_of(def), controller, zone));
    g.state.zone_push(zone, id); g.state.see_statics(); id
}
fn cx(g: &mut RulesGame) -> Cx<'_> { Cx { g, frame: 0, cursor: 0, mana_source: None, entering: None, lineage: Vec::new(), damage: None, asked_at: None, doer: Some(YOU), producing: None, reached: None } }
fn life_events(g: &RulesGame) -> usize { g.log.iter().filter(|r| matches!(r.event, Event::LifeChange { .. })).count() }

#[test]
fn life_a_cost_pays_is_replaced_and_the_cost_is_paid() {
    let mut g = board(); let source = put(&mut g, &PAYER, YOU, Zone::BATTLEFIELD);
    put(&mut g, &PAYMENT_REPLACER, YOU, Zone::BATTLEFIELD);
    let a = AbilityRef { card: g.cards.id_of(&PAYER), index: 0 };
    activate(&mut cx(&mut g), YOU, source, a, 0, &[], None, None, None).unwrap();
    assert_eq!(g.state.stack.len(), 1, "the ability is on the stack: its cost was paid");
    assert_eq!(g.state.player(YOU).life, 21, "no life was lost, and the replacement's gain happened");
    assert!(!g.log.iter().any(|r| matches!(r.event, Event::LifeChange { delta, .. } if delta < 0)), "no life loss is announced");
}
#[test]
fn the_same_cost_without_the_replacement_loses_the_life() {
    let mut g = board(); let source = put(&mut g, &PAYER, YOU, Zone::BATTLEFIELD);
    let a = AbilityRef { card: g.cards.id_of(&PAYER), index: 0 };
    activate(&mut cx(&mut g), YOU, source, a, 0, &[], None, None, None).unwrap();
    assert_eq!(g.state.player(YOU).life, 18);
}
#[test]
fn life_lost_otherwise_is_no_payment() {
    let mut g = board(); put(&mut g, &PAYMENT_REPLACER, YOU, Zone::BATTLEFIELD);
    ops::change_life(&mut cx(&mut g), YOU, -2);
    cx(&mut g).lose_life(YOU, 3);
    cx(&mut g).lose_life_together(&[(YOU, 1)]);
    assert_eq!(g.state.player(YOU).life, 14, "an effect's loss is not paid");
}
#[test]
fn life_paid_by_a_resolving_instruction_is_replaced_too() {
    let mut g = board(); put(&mut g, &PAYMENT_REPLACER, YOU, Zone::BATTLEFIELD);
    let source = put(&mut g, &PAYER, THEM, Zone::BATTLEFIELD);
    let spared = put(&mut g, &PAYER, YOU, Zone::BATTLEFIELD);
    let lost = put(&mut g, &PAYER, YOU, Zone::BATTLEFIELD);
    // "Destroy each permanent unless its controller pays 1 life": one payment is made for `spared`, none for `lost`.
    ops::pay_life_with_permanents(&mut cx(&mut g), source, &[spared, lost], &[(YOU, 1, spared)]);
    assert_eq!(g.state.player(YOU).life, 21, "the payment was replaced and counted as paid");
    assert!(g.state.obj(spared).became.is_none(), "so the permanent it was paid for is spared");
    assert!(g.state.obj(lost).became.is_some(), "and the one no payment was made for is destroyed");
    assert_eq!(life_events(&g), 1, "only the gain is announced");
}
#[test]
fn life_paid_for_every_permanent_is_replaced_when_nothing_is_destroyed() {
    let mut g = board(); put(&mut g, &PAYMENT_REPLACER, YOU, Zone::BATTLEFIELD);
    let source = put(&mut g, &PAYER, THEM, Zone::BATTLEFIELD);
    let spared = put(&mut g, &PAYER, YOU, Zone::BATTLEFIELD);
    // Every permanent is paid for, so nothing is destroyed alongside the payment: the life is still paid, and replaced.
    ops::pay_life_with_permanents(&mut cx(&mut g), source, &[spared], &[(YOU, 1, spared)]);
    assert_eq!(g.state.player(YOU).life, 21, "the payment was replaced and counted as paid");
    assert!(g.state.obj(spared).became.is_none(), "so the permanent it was paid for is spared");
}
#[test]
fn a_replacement_judges_whose_payment_it_is() {
    let mut g = board(); put(&mut g, &PAYMENT_REPLACER, YOU, Zone::BATTLEFIELD);
    ops::pay_life(&mut cx(&mut g), YOU, 5);
    assert_eq!(g.state.player(YOU).life, 21);
    ops::pay_life(&mut cx(&mut g), THEM, 5);
    assert_eq!(g.state.player(THEM).life, 15, "another player's payment is not this replacement's");
}
