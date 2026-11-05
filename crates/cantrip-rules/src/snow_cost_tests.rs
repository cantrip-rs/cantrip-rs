//! CR 107.4h: snow is a source quality, never a mana color or type.
use crate::{Cx, Game, Setup, actions};
use cantrip_api::*;
use cantrip_api::Game as Rules;
use cantrip_state::{Frame, FrameKind};
const YOU: PlayerId = PlayerId(0);
static PLAIN: CardDef = CardDef { name: "Plain source", types: TypeSet::of(&[CardType::Land]), ..CardDef::BLANK };
static SNOW: CardDef = CardDef { name: "Snow source", supertypes: Supertypes { basic: true, legendary: false, snow: true }, ..PLAIN };
fn position(snow: bool) -> (Game, ObjectId) {
    let cards = Box::leak(Box::new(CardTable::new([&PLAIN, &SNOW])));
    let deck = vec![cards.id_of(&PLAIN); 20];
    let mut g = Game::new(cards, &Setup { decks: vec![deck; 2], ..Default::default() });
    while !g.state.agenda.is_empty() { g.state.frame_remove(0); }
    g.state.frame_push(Frame::new(FrameKind::Settle));
    let id = g.state.push_obj(Obj::new(cards.id_of(if snow { &SNOW } else { &PLAIN }), YOU, Zone::BATTLEFIELD));
    g.state.zone_push(Zone::BATTLEFIELD, id);
    (g, id)
}
fn cx(g: &mut Game, source: ObjectId) -> Cx<'_> {
    Cx { g, frame: 0, cursor: 0, mana_source: Some(source), entering: None, lineage: Vec::new(), damage: None, asked_at: None, doer: None, producing: None }
}
#[test]
fn snow_source_mana_pays_a_snow_symbol() {
    let (mut g, source) = position(true);
    cx(&mut g, source).add_mana(YOU, Mana::U, 1);
    let cost = cantrip_api::mana("{S}");
    let paid = actions::pay_mana(&mut cx(&mut g, source), YOU, source, cost).expect("snow source mana pays {S}");
    assert_eq!(paid.get(Mana::U), 1);
    assert_eq!(g.state.player(YOU).pool.get(Mana::U), 0);
}
#[test]
fn ordinary_source_mana_cannot_pay_a_snow_symbol() {
    let (mut g, source) = position(false);
    cx(&mut g, source).add_mana(YOU, Mana::U, 1);
    let cost = cantrip_api::mana("{S}");
    assert!(actions::pay_mana(&mut cx(&mut g, source), YOU, source, cost).is_err());
    assert_eq!(g.state.player(YOU).pool.get(Mana::U), 1);
}
#[test]
fn both_qualities_pay_their_ordinary_color() {
    for snow in [false, true] {
        let (mut g, source) = position(snow);
        cx(&mut g, source).add_mana(YOU, Mana::U, 1);
        let paid = actions::pay_mana(&mut cx(&mut g, source), YOU, source, cantrip_api::mana("{U}")).unwrap();
        assert_eq!(paid.get(Mana::U), 1);
    }
}


#[test]
fn pool_advice_cannot_double_spend_snow_for_a_color_and_snow_symbol() {
    for (snow_type, ordinary_type, payable) in [(Mana::U, Mana::R, false), (Mana::R, Mana::U, true)] {
        let (mut g, source) = position(true);
        cx(&mut g, source).add_mana(YOU, snow_type, 1);
        g.state.add_mana(YOU, ordinary_type, 1, source, None, ManaLasts::Step, false);
        assert_eq!(actions::available(&cx(&mut g, source), YOU).covers(&mana("{U}{S}")), payable);
        let result = actions::pay_mana(&mut cx(&mut g, source), YOU, source, mana("{U}{S}"));
        assert_eq!(result.is_ok(), payable);
    }
}

#[test]
fn captured_quality_survives_the_producer_losing_snow_and_leaving() {
    let (mut g, source) = position(true);
    cx(&mut g, source).add_mana(YOU, Mana::U, 1);
    let plain = g.cards.id_of(&PLAIN);
    g.state.update_obj(source, |o| o.card = plain);
    g.state.see_statics();
    cx(&mut g, source).add_mana(YOU, Mana::U, 1);
    let qualities: Vec<_> = g.state.mana_from_of(YOU).map(|m| (m.snow, m.pool.get(Mana::U))).collect();
    assert_eq!(qualities, vec![(true, 1), (false, 1)]);
    g.state.zone_remove(Zone::BATTLEFIELD, source);
    g.state.update_obj(source, |o| o.zone = Zone::graveyard(YOU));
    g.state.zone_push(Zone::graveyard(YOU), source);
    g.state.see_statics();
    // The snow unit is the only possible assignment; source characteristics at spending do not matter.
    actions::pay_mana(&mut cx(&mut g, source), YOU, source, mana("{S}")).unwrap();
    assert_eq!(g.state.player(YOU).pool.get(Mana::U), 1);
    assert!(g.state.mana_from_of(YOU).all(|m| !m.snow));
}

#[test]
fn snow_cost_is_independent_of_color_mana_value_arithmetic_and_any_type() {
    let c = mana("{X}{2}{S}{S}{U}");
    assert_eq!(c.snow, 2);
    assert_eq!(c.mana_value(), 5);
    assert_eq!(c.to_string(), "{X}{2}{S}{S}{U}");
    assert_eq!(c.plus(&mana("{S}")).snow, 3);
    assert_eq!(c.times(3).unwrap().snow, 6);
    assert_eq!(c.paid_with_any_color().snow, 2);
    assert_eq!(c.paid_with_any_type().snow, 2);
}

#[test]
fn snow_large_payment_has_exact_provenance_without_a_unit_limit() {
    let (mut g, source) = position(true);
    cx(&mut g, source).add_mana(YOU, Mana::U, 300);
    let mut c = ManaCost::ZERO; c.snow = 300;
    let paid = actions::pay_mana(&mut cx(&mut g, source), YOU, source, c).unwrap();
    assert_eq!(paid.get(Mana::U), 300);
    assert_eq!(g.state.player(YOU).pool.get(Mana::U), 0);
    assert!(cx(&mut g, source).history().all().iter().any(|e| matches!(e.event, Event::ManaSpent { snow: 300, .. })));
}

static TAP_SNOW: CardDef = CardDef { name: "Snow tap source", abilities: &[mana_ability(TAP, add(Mana::U), "{T}: Add {U}.")], ..SNOW };
static TAP_PLAIN: CardDef = CardDef { name: "Plain tap source", abilities: &[mana_ability(TAP, add(Mana::R), "{T}: Add {R}.")], ..PLAIN };
static SAC_SNOW: CardDef = CardDef { name: "Snow sacrifice source", abilities: &[mana_ability(TAP.and(SACRIFICE_THIS), add(Mana::U), "{T}, Sacrifice this: Add {U}.")], ..SNOW };
static FILTER: CardDef = CardDef { name: "Snow-cost filter", abilities: &[mana_ability(cost("{S}, {T}"), add_n(Mana::R, 2), "{S}, {T}: Add {R}{R}.")], ..PLAIN };
fn nothing(_: &mut dyn cantrip_api::Game, _: &StackItem) -> Result<(), Suspend> { Ok(()) }
static MIXED: CardDef = CardDef { name: "Snow mixed spell", cost: mana("{U}{S}"), has_cost: true, types: TypeSet::of(&[CardType::Sorcery]), spell: spell(&[], nothing), ..CardDef::BLANK };
fn runtime() -> Game {
    let cards = Box::leak(Box::new(CardTable::new([&PLAIN, &SNOW, &TAP_SNOW, &TAP_PLAIN, &SAC_SNOW, &FILTER, &MIXED, &REPLACER, &SNOW_REPLACER, &FLARE, &SNOW_FLARE, &REDUCER, &REDUCED, &HYBRID, &ADDITIONAL, &X_SNOW, &ALTERNATIVE, &ARTIFACT])));
    let deck = vec![cards.id_of(&PLAIN); 20];
    let mut g = Game::new(cards, &Setup { decks: vec![deck; 2], ..Default::default() });
    g.verify = true;
    while !g.state.agenda.is_empty() { g.state.frame_remove(0); }
    g.state.update_turn(|t| { t.active = YOU; t.priority = YOU; t.set_step(Step::PrecombatMain); });
    g.state.frame_push(Frame::new(FrameKind::Priority));
    g
}
fn put(g: &mut Game, def: &'static CardDef, zone: Zone) -> ObjectId {
    let id = g.state.push_obj(Obj::new(g.cards.id_of(def), YOU, zone));
    g.state.zone_push(zone, id); g.state.see_statics(); id
}
fn options(g: &mut Game) -> Vec<ActionOption> {
    g.run();
    match &g.pending().expect("window").kind {
        RequestKind::Priority { options } | RequestKind::PayCosts { options, .. } => options.clone(),
        other => panic!("unexpected {other:?}"),
    }
}
fn choose(g: &mut Game, pred: impl Fn(&ActionOption)->bool) {
    let opts = options(g);
    let action = Action::of(opts.iter().find(|o| pred(o)).expect("offered action"));
    let r = g.pending().unwrap().clone();
    g.answer(r.id, r.player, Answer::Act(action)).unwrap(); g.run();
    while let Some(Request { kind: RequestKind::ChooseCostPayment { choices, .. }, .. }) = g.pending() {
        let chosen = choices.iter().position(|c| c.kind == CostPaymentKind::Tap)
            .or_else(|| choices.iter().position(|c| c.kind == CostPaymentKind::Mana)).unwrap_or(0);
        let r = g.pending().cloned().unwrap();
        g.answer(r.id, r.player, Answer::CostPayment(chosen)).unwrap(); g.run();
    }
}
fn close_mana_window(g: &mut Game) {
    if matches!(g.pending().map(|r| &r.kind), Some(RequestKind::PayCosts { mana_abilities: true, .. })) {
        choose(g, |o| o.kind == ActionKind::ConfirmCasting);
    }
}
#[test]
fn snow_ability_advice_nested_cost_and_sacrificed_source_are_exact() {
    let mut g = runtime();
    let snow = put(&mut g, &TAP_SNOW, Zone::BATTLEFIELD);
    let filter = put(&mut g, &FILTER, Zone::BATTLEFIELD);
    let plain = put(&mut g, &TAP_PLAIN, Zone::BATTLEFIELD);
    assert!(actions::available(&cx(&mut g, snow), YOU).covers(&mana("{S}")));
    assert!(!actions::available(&cx(&mut g, snow), YOU).covers(&mana("{S}{S}")));
    choose(&mut g, |o| o.source == Some(filter));
    choose(&mut g, |o| o.source == Some(snow));
    close_mana_window(&mut g);
    assert_eq!(g.state.player(YOU).pool.get(Mana::R), 2);
    assert!(g.state.mana_from_of(YOU).all(|m| !m.snow), "filter, not payment producer, is source of new mana");
    assert!(!g.state.obj(plain).tapped);
    let mut g = runtime(); let sacrifice = put(&mut g, &SAC_SNOW, Zone::BATTLEFIELD);
    choose(&mut g, |o| o.source == Some(sacrifice));
    assert!(!g.state.battlefield().contains(&sacrifice));
    assert!(g.state.mana_from_of(YOU).any(|m| m.snow && m.source == sacrifice));
}
#[test]
fn allocation_choices_validate_cancel_undo_and_goto_without_losing_quality() {
    let mut g = runtime();
    let snow = put(&mut g, &TAP_SNOW, Zone::BATTLEFIELD);
    let spell = put(&mut g, &MIXED, Zone::hand(YOU));
    g.state.add_mana(YOU, Mana::U, 2, snow, None, ManaLasts::Step, true);
    g.state.add_mana(YOU, Mana::R, 1, snow, None, ManaLasts::Step, true);
    choose(&mut g, |o| o.kind == ActionKind::CastSpell && o.source == Some(spell));
    close_mana_window(&mut g);
    let opts = options(&mut g);
    assert!(opts.iter().any(|o| o.kind == ActionKind::AssignMana && o.snow && o.pay_snow));
    assert!(opts.iter().any(|o| o.kind == ActionKind::AssignMana && o.snow && !o.pay_snow));
    let request = g.pending().unwrap().clone();
    let mut forged = Action::of(opts.iter().find(|o| o.kind == ActionKind::AssignMana && o.pay_snow).unwrap());
    forged.snow = false;
    assert!(g.answer(request.id, request.player, Answer::Act(forged)).is_err());
    let node = g.at;
    choose(&mut g, |o| o.kind == ActionKind::AssignMana && o.pay_snow);
    assert!(g.state.stack.iter().any(|i| g.state.obj(i.object).card == g.cards.id_of(&MIXED)));
    let paid = g.at;
    g.goto(node);
    assert_eq!(g.state.player(YOU).pool.get(Mana::U), 2);
    assert!(g.state.mana_from_of(YOU).all(|m| m.snow));
    g.goto(paid);
    assert_eq!(g.state.player(YOU).pool.get(Mana::U), 0);
    g.goto(node);
    choose(&mut g, |o| o.kind == ActionKind::CancelCasting);
    assert_eq!(g.state.player(YOU).pool.get(Mana::U), 2);
    assert!(g.state.obj(spell).zone.is(ZoneKind::Hand));
    // Real mana ability undo uses the same provenance journal.
    let before_mana = g.checkpoint();
    choose(&mut g, |o| o.kind == ActionKind::ActivateAbility && o.source == Some(snow));
    assert_eq!(g.state.player(YOU).pool.get(Mana::U), 3);
    g.goto(before_mana);
    assert_eq!(g.state.player(YOU).pool.get(Mana::U), 2);
    assert!(!g.state.obj(snow).tapped);
}

fn replace_any(_: &dyn Query, _: &Asker, p: &Proposal) -> bool { matches!(p, Proposal::ManaProduction(_)) }
fn replace_colorless(_: &dyn Query, _: &Asker, p: &mut Proposal) {
    if let Proposal::ManaProduction(p) = p { p.mana = ManaPool::default(); p.mana.0[Mana::C as usize] = 2; }
}
static REPLACER: CardDef = CardDef { name: "Plain production replacement", replacements: &[replacement(ReplacementKind::ManaProduction, replace_any, replace_colorless, "produces {C}{C} instead")], ..PLAIN };
static SNOW_REPLACER: CardDef = CardDef { name: "Snow production replacement", supertypes: SNOW.supertypes, ..REPLACER };
static FLARE: CardDef = CardDef { name: "Plain mana trigger", abilities: &[triggered(when(EventMask::TAPPED, cantrip_mechanics::mana::a_player_taps_a_land_for_mana), cantrip_mechanics::mana::add_one_mana_of_any_type_that_land_produced, "Whenever a land is tapped for mana, add one mana of a type it produced.").mana_ability(TriggeredMana::OneOfProduced)], ..PLAIN };
static SNOW_FLARE: CardDef = CardDef { name: "Snow mana trigger", supertypes: SNOW.supertypes, ..FLARE };
#[test]
fn replacement_mana_keeps_original_producer_quality_and_triggered_mana_uses_its_own() {
    for (producer, replacement, expected) in [(&TAP_SNOW, &REPLACER, true), (&TAP_PLAIN, &SNOW_REPLACER, false)] {
        let mut g = runtime(); let land = put(&mut g, producer, Zone::BATTLEFIELD);
        put(&mut g, replacement, Zone::BATTLEFIELD);
        choose(&mut g, |o| o.kind == ActionKind::ActivateAbility && o.source == Some(land));
        assert_eq!(g.state.player(YOU).pool.get(Mana::C), 2);
        assert!(g.state.mana_from_of(YOU).all(|m| m.snow == expected && m.source == land));
    }
    for (producer, trigger) in [(&TAP_SNOW, &FLARE), (&TAP_PLAIN, &SNOW_FLARE)] {
        let mut g = runtime(); let land = put(&mut g, producer, Zone::BATTLEFIELD); let flare = put(&mut g, trigger, Zone::BATTLEFIELD);
        choose(&mut g, |o| o.kind == ActionKind::ActivateAbility && o.source == Some(land));
        assert_eq!(g.state.mana_from_of(YOU).filter(|m| m.snow).map(|m| m.pool.total()).sum::<u32>(), 1);
        assert_eq!(g.state.mana_from_of(YOU).filter(|m| !m.snow).map(|m| m.pool.total()).sum::<u32>(), 1);
        assert!(g.state.mana_from_of(YOU).any(|m| m.source == flare && m.snow == trigger.supertypes.snow));
    }
}
fn reduce(_: &dyn Query, _: ObjectId, _: PlayerId, _: Costed, _: &[Target]) -> CostDelta { CostDelta::less(10) }
static REDUCER: CardDef = CardDef { name: "Generic-only reducer", cost_changes: &[reduce], ..PLAIN };
static REDUCED: CardDef = CardDef { name: "Reduced snow spell", cost: mana("{2}{S}"), ..MIXED };
static HYBRID: CardDef = CardDef { name: "Hybrid snow spell", cost: mana("{U/R}{S}"), ..MIXED };
static ADDITIONAL: CardDef = CardDef { name: "Additional snow spell", cost: mana("{U}"), additional_cost: Some(Cost::mana(mana("{S}"))), ..MIXED };
static X_SNOW: CardDef = CardDef { name: "X snow spell", cost: mana("{X}{S}"), ..MIXED };
#[test]
fn generic_reduction_never_reduces_snow_and_hybrid_additional_x_costs_keep_it() {
    for quality in [false, true] {
        let mut g = runtime(); let spell = put(&mut g, &REDUCED, Zone::hand(YOU));
        let source = put(&mut g, &TAP_PLAIN, Zone::BATTLEFIELD); put(&mut g, &REDUCER, Zone::BATTLEFIELD);
        g.state.add_mana(YOU, Mana::C, 1, source, None, ManaLasts::Step, quality);
        assert_eq!(options(&mut g).iter().any(|o| o.kind == ActionKind::CastSpell && o.source == Some(spell)), quality);
        if quality { choose(&mut g, |o| o.kind == ActionKind::CastSpell && o.source == Some(spell)); close_mana_window(&mut g); assert_eq!(g.state.player(YOU).pool.total(), 0); }
    }
    for def in [&HYBRID, &ADDITIONAL] {
        let mut g = runtime(); let spell = put(&mut g, def, Zone::hand(YOU));
        let source = put(&mut g, &PLAIN, Zone::BATTLEFIELD);
        g.state.add_mana(YOU, Mana::U, 1, source, None, ManaLasts::Step, false);
        g.state.add_mana(YOU, Mana::G, 1, source, None, ManaLasts::Step, true);
        choose(&mut g, |o| o.kind == ActionKind::CastSpell && o.source == Some(spell));
        close_mana_window(&mut g);
        assert_eq!(g.state.player(YOU).pool.total(), 0, "{} pending {:?}", def.name, g.pending());
    }
    let mut g = runtime(); let spell = put(&mut g, &X_SNOW, Zone::hand(YOU)); let source = put(&mut g, &PLAIN, Zone::BATTLEFIELD);
    g.state.add_mana(YOU, Mana::C, 1, source, None, ManaLasts::Step, true);
    g.state.add_mana(YOU, Mana::C, 2, source, None, ManaLasts::Step, false);
    choose(&mut g, |o| o.kind == ActionKind::CastSpell && o.source == Some(spell));
    let r = g.pending().unwrap().clone();
    g.answer(r.id, YOU, Answer::Announce(2)).unwrap(); g.run();
    close_mana_window(&mut g);
    assert_eq!(g.state.player(YOU).pool.total(), 0);
}

#[test]
fn snow_cost_composition_and_partial_resolution_advice_are_not_generic() {
    assert_eq!(Cost::mana(mana("{S}")).and(Cost::mana(mana("{S}"))).mana.snow, 2);
    assert_eq!(Cost::mana(mana("{S}")).times(3).mana.snow, 3);
    assert_eq!(Cost::mana(mana("{S}")).times(0).mana.snow, 0);
    static QUESTION: Question = Question::new("Draw", "pay {cost}?", "", "Pay", "Decline");
    for snow in [false, true] {
        let (mut g, source) = position(snow);
        cx(&mut g, source).add_mana(YOU, Mana::U, 1);
        assert!(actions::may_pay(&mut cx(&mut g, source), YOU, source, source, &Cost::mana(mana("{S}{S}")), &QUESTION).is_err());
        let RequestKind::UnlessPays { remaining, options, .. } = &g.pending().unwrap().kind else { panic!("unless") };
        assert_eq!(remaining.snow, if snow { 1 } else { 2 });
        assert!(!options.iter().any(|o| o.kind == ActionKind::PayCost));
    }
}

static ALTERNATIVE: CardDef = CardDef { name: "Alternative snow spell", cost: mana("{5}"), ways: &[way(Cost::mana(mana("{S}")), ZoneKind::Hand, "Snow alternative {S}")], ..MIXED };
static ARTIFACT: CardDef = CardDef { name: "Restricted snow artifact", cost: mana("{S}"), types: TypeSet::of(&[CardType::Artifact]), ..MIXED };
#[test]
fn alternative_cost_and_restricted_snow_payments_keep_both_requirements() {
    for quality in [false, true] {
        let mut g = runtime(); let spell = put(&mut g, &ALTERNATIVE, Zone::hand(YOU)); let source = put(&mut g, &TAP_PLAIN, Zone::BATTLEFIELD);
        g.state.add_mana(YOU, Mana::U, 1, source, None, ManaLasts::Step, quality);
        assert_eq!(options(&mut g).iter().any(|o| o.kind == ActionKind::CastSpell && o.source == Some(spell) && o.index == 1), quality);
        if quality { choose(&mut g, |o| o.source == Some(spell) && o.index == 1); close_mana_window(&mut g); assert_eq!(g.state.player(YOU).pool.total(), 0); }
    }
    let mut g = runtime(); let source = put(&mut g, &SNOW, Zone::BATTLEFIELD);
    put(&mut g, &REDUCER, Zone::BATTLEFIELD);
    let artifact = put(&mut g, &ARTIFACT, Zone::hand(YOU)); let sorcery = put(&mut g, &REDUCED, Zone::hand(YOU));
    cx(&mut g, source).add_restricted_mana(YOU, Mana::U, 1, ManaRestriction::ARTIFACT_SPELLS);
    assert!(!options(&mut g).iter().any(|o| o.kind == ActionKind::CastSpell && o.source == Some(sorcery)));
    choose(&mut g, |o| o.kind == ActionKind::CastSpell && o.source == Some(artifact));
    close_mana_window(&mut g);
    assert!(!g.state.has_restricted_mana(YOU));
}

#[test]
fn phyrexian_life_payment_cannot_replace_the_snow_requirement() {
    static QUESTION: Question = Question::new("Draw", "pay {cost}?", "", "Pay", "Decline");
    for snow in [false, true] {
        let (mut g, source) = position(snow);
        cx(&mut g, source).add_mana(YOU, Mana::G, 1);
        assert!(actions::may_pay(&mut cx(&mut g, source), YOU, source, source, &Cost::mana(mana("{S}{U/P}")), &QUESTION).is_err());
        let RequestKind::UnlessPays { options, remaining, .. } = &g.pending().unwrap().kind else { panic!("unless") };
        assert_eq!(options.iter().any(|o| o.kind == ActionKind::PayCost), snow);
        assert_eq!(remaining.snow, if snow { 0 } else { 1 });
        assert_eq!(g.state.player(YOU).life, 20, "advice does not pay anything");
    }
}
