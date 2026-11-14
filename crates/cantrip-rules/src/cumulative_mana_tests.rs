//! Restricted cumulative-upkeep mana is accepted only for the keyword's cost (CR 106.6, 702.24a).
use super::*;
use crate::Game as RulesGame;
use cantrip_state::{Frame, FrameKind};
use std::sync::OnceLock;

const YOU: PlayerId = PlayerId(0);
const THEM: PlayerId = PlayerId(1);
const ASK: Question = Question::new("OtherPayment", "pay?", "", "Pay", "Decline");
fn upkeep(g: &mut dyn cantrip_api::Game, item: &StackItem) -> Result<(), Suspend> {
    cantrip_mechanics::cumulative_upkeep::resolve(g, item, cantrip_mechanics::cumulative_upkeep::Payment::Mana(mana("{U}")))
}
fn upkeep_with_life(g: &mut dyn cantrip_api::Game, item: &StackItem) -> Result<(), Suspend> {
    cantrip_mechanics::cumulative_upkeep::resolve(g, item, cantrip_mechanics::cumulative_upkeep::Payment::ManaAndLife(mana("{U}"), 1))
}
fn never(_: &dyn Query, _: ObjectId, _: &Event) -> Option<Fired> { None }
static SOURCE: CardDef = CardDef { name: "Cumulative mana upkeep probe", types: TypeSet::of(&[CardType::Enchantment]),
    abilities: &[triggered(when(EventMask::ZONE_CHANGE, never), upkeep, "Cumulative upkeep {U}.")], ..CardDef::BLANK };
static LIFE_SOURCE: CardDef = CardDef { name: "Cumulative mana and life upkeep probe", types: TypeSet::of(&[CardType::Enchantment]),
    abilities: &[triggered(when(EventMask::ZONE_CHANGE, never), upkeep_with_life, "Cumulative upkeep {U} and 1 life.")], ..CardDef::BLANK };
static PRODUCER: CardDef = CardDef { name: "Cumulative mana producer probe", types: TypeSet::of(&[CardType::Artifact]),
    abilities: &[mana_ability(TAP, add(Mana::U).only_for(ManaRestriction::CumulativeUpkeep), "{T}: Add {U} for cumulative upkeep.")], ..CardDef::BLANK };
static FILTER: CardDef = CardDef { name: "Cumulative mana filter probe", types: TypeSet::of(&[CardType::Artifact]),
    abilities: &[mana_ability(cost("{U}, {T}"), add_n(Mana::U, 2), "{U}, {T}: Add {U}{U}.")], ..CardDef::BLANK };
fn board() -> (RulesGame, ObjectId, ObjectId) { board_with(&SOURCE) }
fn board_with(def: &'static CardDef) -> (RulesGame, ObjectId, ObjectId) {
    static TABLE: OnceLock<CardTable> = OnceLock::new();
    let cards = TABLE.get_or_init(|| CardTable::new([&SOURCE, &LIFE_SOURCE, &PRODUCER, &FILTER]));
    let mut g = RulesGame::new(cards, &crate::Setup { decks: vec![vec![],vec![]], ..Default::default() });
    while !g.state.agenda.is_empty() { g.state.frame_remove(0); }
    g.state.update_turn(|t| { t.active = YOU; t.priority = YOU; t.set_step(Step::Upkeep); });
    let source = g.state.push_obj(Obj::new(cards.id_of(def), YOU, Zone::BATTLEFIELD));
    g.state.zone_push(Zone::BATTLEFIELD, source);
    let producer = g.state.push_obj(Obj::new(cards.id_of(&PRODUCER), YOU, Zone::BATTLEFIELD));
    g.state.zone_push(Zone::BATTLEFIELD, producer);
    let item = crate::actions::stack_item(StackKind::Triggered, source, AbilityRef { card: cards.id_of(def), index: 0 }, YOU, &[]);
    g.state.stack_push(item);
    let serial = g.state.stack.last().unwrap().serial;
    g.state.frame_push(Frame::new(FrameKind::Resolve { serial }));
    (g, source, producer)
}
fn cx(g: &mut RulesGame) -> Cx<'_> {
    Cx { g, frame: 0, cursor: 0, mana_source: None, entering: None, lineage: vec![], damage: None, asked_at: None, doer: None, producing: None, reached: None }
}
fn answer(g: &mut RulesGame, kind: ActionKind, source: Option<ObjectId>) {
    let ask = g.pending().cloned().expect("question");
    let RequestKind::UnlessPays { options, .. } = &ask.kind else { panic!("expected cost question") };
    let option = options.iter().find(|o| o.kind == kind && (source.is_none() || o.source == source)).expect("offered action");
    g.answer(ask.id, YOU, Answer::Act(Action::of(option))).unwrap();
}
#[test]
fn keyword_accepts_restricted_pool_and_preserves_payer_and_undo() {
    let (mut g, source, producer) = board();
    g.state.add_mana(YOU, Mana::U, 1, producer, Some(ManaRestriction::CumulativeUpkeep), ManaLasts::Step, false);
    g.run();
    let ask = g.pending().unwrap();
    let RequestKind::UnlessPays { options, .. } = &ask.kind else { panic!("payment") };
    assert!(options.iter().any(|o| o.kind == ActionKind::PayCost));
    let before = g.at;
    answer(&mut g, ActionKind::PayCost, None);
    let after = g.at;
    assert_eq!(g.obj(source).counters(CounterKind::Age), 1);
    assert!(g.state.restricted_of(YOU).all(|(_, p)| p.is_empty()));
    assert!(g.log.iter().any(|r| matches!(r.event, Event::ManaSpent { player: YOU, source: p, on, .. } if p == producer && on == source)));
    g.goto(before);
    assert_eq!(g.state.restricted_of(YOU).next().unwrap().1.get(Mana::U), 1);
    g.goto(after);
    assert!(g.state.restricted_of(YOU).all(|(_, p)| p.is_empty()));
}
#[test]
fn mana_ability_is_offered_for_keyword_and_other_cost_cannot_spend_it() {
    let (mut g, source, producer) = board();
    g.run();
    let RequestKind::UnlessPays { options, .. } = &g.pending().unwrap().kind else { panic!("payment") };
    let ability = options.iter().find(|o| o.source == Some(producer)).unwrap();
    assert!(!ability.pays_nothing);
    answer(&mut g, ActionKind::ActivateAbility, Some(producer));
    answer(&mut g, ActionKind::PayCost, None);
    assert_eq!(g.obj(source).counters(CounterKind::Age), 1);

    let (mut g, source, producer) = board();
    g.state.add_mana(YOU, Mana::U, 1, producer, Some(ManaRestriction::CumulativeUpkeep), ManaLasts::Step, false);
    assert!(crate::actions::may_pay(&mut cx(&mut g), YOU, source, source, &Cost::mana(mana("{U}")), &ASK).is_err());
    let RequestKind::UnlessPays { options, .. } = &g.pending().unwrap().kind else { panic!("payment") };
    assert!(!options.iter().any(|o| o.kind == ActionKind::PayCost));
    assert!(options.iter().find(|o| o.source == Some(producer)).unwrap().pays_nothing);
    assert_eq!(g.state.player(THEM).pool.get(Mana::U), 0);
}

#[test]
fn age_cost_combines_ordinary_and_restricted_mana() {
    let (mut g, source, producer) = board();
    crate::counters::put(&mut cx(&mut g), source, CounterKind::Age, 1, YOU, crate::counters::How::Effect);
    g.state.add_mana(YOU, Mana::U, 1, producer, Some(ManaRestriction::CumulativeUpkeep), ManaLasts::Step, false);
    g.state.add_mana(YOU, Mana::U, 1, producer, None, ManaLasts::Step, false);
    g.run();
    let RequestKind::UnlessPays { total, options, .. } = &g.pending().unwrap().kind else { panic!("payment") };
    assert_eq!(*total, mana("{U}{U}"));
    assert!(options.iter().any(|o| o.kind == ActionKind::PayCost));
    answer(&mut g, ActionKind::PayCost, None);
    assert_eq!(g.obj(source).counters(CounterKind::Age), 2);
    assert_eq!(g.state.player(YOU).pool.get(Mana::U), 0);
    assert!(g.state.restricted_of(YOU).all(|(_, p)| p.is_empty()));
}

#[test]
fn ordinary_mana_remains_payable_for_a_non_upkeep_cost() {
    let (mut g, source, producer) = board();
    g.state.add_mana(YOU, Mana::U, 1, producer, None, ManaLasts::Step, false);
    assert!(crate::actions::may_pay(&mut cx(&mut g), YOU, source, source, &Cost::mana(mana("{U}")), &ASK).is_err());
    let RequestKind::UnlessPays { options, .. } = &g.pending().unwrap().kind else { panic!("payment") };
    assert!(options.iter().any(|o| o.kind == ActionKind::PayCost));
}

#[test]
fn restriction_is_rejected_for_spells_and_activations_and_union_remains_additive() {
    let r = ManaRestriction::CumulativeUpkeep;
    assert!(r.may_pay(&ManaPayment::CumulativeUpkeep));
    assert!(!r.may_pay(&ManaPayment::Other));
    assert!(!r.may_pay(&ManaPayment::PlayerAbility));
    assert!(!r.may_pay(&ManaPayment::Ability { source: TypeSet::NONE, subtypes: SubtypeSet::NONE, on_battlefield: true, equip: false }));
    let spell = ManaPayment::Spell { types: TypeSet::of(&[CardType::Creature]), subtypes: SubtypeSet::NONE, legendary: false, mana_value: 1, x: false, from: ZoneKind::Hand, commander: false, card: ObjectId::NONE };
    assert!(!r.may_pay(&spell));
    static PARTS: [ManaRestriction; 2] = [ManaRestriction::CumulativeUpkeep, ManaRestriction::CREATURE_SPELLS];
    let union = ManaRestriction::AnyOf(&PARTS);
    assert!(union.may_pay(&ManaPayment::CumulativeUpkeep));
    assert!(union.may_pay(&spell));
    assert!(!union.may_pay(&ManaPayment::Other));
}

#[test]
fn a_mana_ability_cannot_use_upkeep_only_mana_for_its_own_activation() {
    let (mut g, source, producer) = board();
    let filter = g.state.push_obj(Obj::new(g.cards.id_of(&FILTER), YOU, Zone::BATTLEFIELD));
    g.state.zone_push(Zone::BATTLEFIELD, filter);
    g.state.update_obj(producer, |o| o.tapped = true);
    g.state.add_mana(YOU, Mana::U, 1, producer, Some(ManaRestriction::CumulativeUpkeep), ManaLasts::Step, false);
    g.run();
    let RequestKind::UnlessPays { options, .. } = &g.pending().unwrap().kind else { panic!("payment") };
    assert!(!options.iter().any(|o| o.source == Some(filter)), "activation cost is not cumulative upkeep");
    assert!(options.iter().any(|o| o.kind == ActionKind::PayCost), "the outer upkeep may use it");
    answer(&mut g, ActionKind::PayCost, None);
    assert_eq!(g.obj(source).counters(CounterKind::Age), 1);
}

#[test]
fn ordinary_and_restricted_units_of_one_color_remain_a_choice() {
    let (mut g, source, producer) = board();
    g.state.add_mana(YOU, Mana::U, 1, producer, Some(ManaRestriction::CumulativeUpkeep), ManaLasts::Step, false);
    g.state.add_mana(YOU, Mana::U, 1, producer, None, ManaLasts::Step, false);
    g.run();
    answer(&mut g, ActionKind::PayCost, None);
    let ask = g.pending().cloned().unwrap();
    let RequestKind::PayCosts { options, .. } = &ask.kind else { panic!("mana choice") };
    assert!(options.iter().any(|o| o.kind == ActionKind::AssignMana && o.restriction.is_none()));
    let restricted = options.iter().find(|o| o.kind == ActionKind::AssignMana && o.restriction == Some(ManaRestriction::CumulativeUpkeep)).unwrap();
    g.answer(ask.id, YOU, Answer::Act(Action::of(restricted))).unwrap();
    assert_eq!(g.obj(source).counters(CounterKind::Age), 1);
    assert!(g.log.iter().any(|r| matches!(r.event, Event::ManaSpent { player: YOU, source: p, on, .. } if p == producer && on == source)));
}

#[test]
fn snow_quality_and_restriction_both_apply_to_the_upkeep_payment() {
    for snow in [false, true] {
        let (mut g, source, producer) = board();
        g.state.add_mana(YOU, Mana::U, 1, producer, Some(ManaRestriction::CumulativeUpkeep), ManaLasts::Step, snow);
        assert!(crate::actions::may_pay_cumulative_upkeep(&mut cx(&mut g), YOU, source, source, &Cost::mana(mana("{S}")), &ASK).is_err());
        let RequestKind::UnlessPays { options, .. } = &g.pending().unwrap().kind else { panic!("payment") };
        assert_eq!(options.iter().any(|o| o.kind == ActionKind::PayCost), snow);
        if snow {
            let pay = options.iter().find(|o| o.kind == ActionKind::PayCost).unwrap();
            let answer = Answer::Act(Action::of(pay));
            g.state.update_frame(0, |f| { f.answers.push((Some(YOU), answer)); f.pending = None; });
            assert_eq!(crate::actions::may_pay_cumulative_upkeep(&mut cx(&mut g), YOU, source, source, &Cost::mana(mana("{S}")), &ASK), Ok(true));
            assert!(g.state.restricted_of(YOU).all(|(_, p)| p.is_empty()));
        }
    }
}

#[test]
fn the_trigger_controller_pays_after_control_of_the_source_changes() {
    let (mut g, source, producer) = board();
    g.state.update_obj(source, |o| o.controller = THEM);
    g.state.add_mana(YOU, Mana::U, 1, producer, Some(ManaRestriction::CumulativeUpkeep), ManaLasts::Step, false);
    g.run();
    assert_eq!(g.pending().unwrap().player, YOU);
    answer(&mut g, ActionKind::PayCost, None);
    assert_eq!(g.obj(source).counters(CounterKind::Age), 1);
    assert!(g.log.iter().any(|r| matches!(r.event, Event::ManaSpent { player: YOU, on, .. } if on == source)));
}

#[test]
fn combined_mana_and_life_keyword_cost_spends_restricted_mana() {
    let (mut g, source, producer) = board_with(&LIFE_SOURCE);
    g.state.add_mana(YOU, Mana::U, 1, producer, Some(ManaRestriction::CumulativeUpkeep), ManaLasts::Step, false);
    g.run();
    let RequestKind::UnlessPays { options, life, .. } = &g.pending().unwrap().kind else { panic!("payment") };
    assert_eq!(*life, 1);
    assert!(options.iter().any(|o| o.kind == ActionKind::PayCost));
    answer(&mut g, ActionKind::PayCost, None);
    assert_eq!(g.state.player(YOU).life, 19);
    assert_eq!(g.obj(source).counters(CounterKind::Age), 1);
    assert!(g.log.iter().any(|r| matches!(r.event, Event::ManaSpent { player: YOU, source: p, on, .. } if p == producer && on == source)));
}

#[test]
fn cast_and_activation_affordability_exclude_upkeep_only_mana() {
    let (mut g, source, producer) = board();
    g.state.update_obj(producer, |o| o.tapped = true);
    let spell = g.state.push_obj(Obj::new(g.cards.id_of(&SOURCE), YOU, Zone::STACK));
    g.state.zone_push(Zone::STACK, spell);
    g.state.add_mana(YOU, Mana::U, 1, producer, Some(ManaRestriction::CumulativeUpkeep), ManaLasts::Step, false);
    assert!(!crate::actions::can_afford(&cx(&mut g), YOU, spell, &mana("{U}")));
    assert!(!crate::actions::can_afford(&cx(&mut g), YOU, source, &mana("{U}")));
    g.state.add_mana(YOU, Mana::U, 1, producer, None, ManaLasts::Step, false);
    assert!(crate::actions::can_afford(&cx(&mut g), YOU, spell, &mana("{U}")));
    assert!(crate::actions::can_afford(&cx(&mut g), YOU, source, &mana("{U}")));
}

#[test]
fn hybrid_and_phyrexian_advice_use_the_keyword_payment_purpose() {
    let (mut g, source, producer) = board();
    g.state.update_obj(producer, |o| o.tapped = true);
    g.state.add_mana(YOU, Mana::C, 2, producer, Some(ManaRestriction::CumulativeUpkeep), ManaLasts::Step, false);
    assert!(crate::actions::may_pay_cumulative_upkeep(&mut cx(&mut g), YOU, source, source, &Cost::mana(mana("{2/U}")), &ASK).is_err());
    let RequestKind::ChooseModes { affordable, .. } = &g.pending().unwrap().kind else { panic!("hybrid announcement") };
    assert_eq!(affordable, &[0], "two upkeep-only mana can pay the generic half");

    let (mut g, source, producer) = board();
    g.state.update_obj(producer, |o| o.tapped = true);
    g.state.add_mana(YOU, Mana::U, 1, producer, Some(ManaRestriction::CumulativeUpkeep), ManaLasts::Step, false);
    let cost = Cost::mana(mana("{U/P}"));
    assert!(crate::actions::may_pay_cumulative_upkeep(&mut cx(&mut g), YOU, source, source, &cost, &ASK).is_err());
    let RequestKind::UnlessPays { options, .. } = &g.pending().unwrap().kind else { panic!("payment") };
    let pay = Answer::Act(Action::of(options.iter().find(|o| o.kind == ActionKind::PayCost).unwrap()));
    g.state.update_frame(0, |f| { f.answers.push((Some(YOU), pay)); f.pending = None; });
    assert!(crate::actions::may_pay_cumulative_upkeep(&mut cx(&mut g), YOU, source, source, &cost, &ASK).is_err());
    let RequestKind::ChooseModes { affordable, .. } = &g.pending().unwrap().kind else { panic!("phyrexian announcement") };
    assert!(affordable.is_empty(), "restricted blue pays without life");
}
