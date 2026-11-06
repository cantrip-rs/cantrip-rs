//! CR 118.11, 118.12a: the Chants' counter payment is a cost on a chosen creature,
//! with a payer and recipient distinct from the enchantment's resolving ability.
use crate::{Cx, Game, Setup};
use cantrip_api::*;
use cantrip_state::{Frame, FrameKind};

const YOU: PlayerId = PlayerId(0);
const PAYER: PlayerId = PlayerId(1);
const ASK: Question = Question::new("PutCounter", "put a -1/-1 counter on {subject}, or take 3 damage?", "", "Put a counter", "Take damage");
const CHOOSE_COST: Question = Question::new("PutCounter", "put a -1/-1 counter on a creature you control, or take 3 damage?", "", "Put a counter", "Take damage");
fn never(_: &dyn Query, _: ObjectId, _: &Event) -> Option<Fired> { None }
fn resolve_cost(g: &mut dyn cantrip_api::Game, item: &StackItem) -> Result<(), Suspend> {
    let candidates = g.permanents_of(PAYER).into_iter().filter(|&id| g.chars(id).is(CardType::Creature)).collect();
    if cantrip_mechanics::counters::may_pay_on_one(g, PAYER, item.object, CounterKind::MinusOne, 1, candidates, &CHOOSE_COST)?.is_some() {
        return Ok(())
    }
    g.damage(item.object, Target::Player(PAYER), 3); Ok(())
}
static CHANT: CardDef = CardDef { name: "Tourach counter cost source", types: TypeSet::of(&[CardType::Enchantment]),
    abilities: &[triggered(when(EventMask::ZONE_CHANGE, never), resolve_cost, "Deal 3 damage unless that player puts a -1/-1 counter on a creature they control.")], ..CardDef::BLANK };
static CREATURE: CardDef = CardDef { name: "Counter cost Ornithopter", types: TypeSet::of(&[CardType::Creature]), power: Some(0), toughness: Some(2), ..CardDef::BLANK };
fn on_yours(_: &dyn Query, asker: &Asker, proposal: &Proposal) -> bool {
    matches!(proposal, Proposal::Counters(c) if c.effect && c.controller == asker.controller && c.counts.iter().any(|&n| n > 0))
}
fn twice(_: &dyn Query, _: &Asker, proposal: &mut Proposal) {
    if let Proposal::Counters(c) = proposal { c.counts = c.counts.map(|n| n.saturating_mul(2)); }
}
static SEASON: CardDef = CardDef { name: "Counter cost Doubling Season", types: TypeSet::of(&[CardType::Enchantment]),
    replacements: &[replacement(ReplacementKind::Counters, on_yours, twice, "If an effect puts counters on your permanent, double them.")], ..CardDef::BLANK };
fn any_put_by_you(_: &dyn Query, asker: &Asker, proposal: &Proposal) -> bool {
    matches!(proposal, Proposal::Counters(c) if c.putter == asker.controller && c.counts.iter().any(|&n| n > 0))
}
fn another_puts(_: &dyn Query, asker: &Asker, proposal: &Proposal) -> bool {
    matches!(proposal, Proposal::Counters(c) if c.putter != asker.controller && c.counts.iter().any(|&n| n > 0))
}
fn half(_: &dyn Query, _: &Asker, proposal: &mut Proposal) {
    if let Proposal::Counters(c) = proposal { c.counts = c.counts.map(|n| n / 2); }
}
static VORINCLEX: CardDef = CardDef { name: "Counter cost Vorinclex", types: TypeSet::of(&[CardType::Creature]), power: Some(6), toughness: Some(6),
    replacements: &[replacement(ReplacementKind::Counters, any_put_by_you, twice, "If you put counters, twice instead."),
        replacement(ReplacementKind::Counters, another_puts, half, "If an opponent puts counters, half rounded down instead.")], ..CardDef::BLANK };
fn ban(_: &dyn Query, asker: &Asker, _: ObjectId, controller: PlayerId, kind: CounterKind) -> bool { controller == asker.controller && kind == CounterKind::MinusOne }
static BAN: CardDef = CardDef { name: "Counter cost Melira", types: TypeSet::of(&[CardType::Enchantment]), counter_bans: &[ban], ..CardDef::BLANK };
fn ancient_cost(g: &mut dyn cantrip_api::Game, item: &StackItem) -> Result<(), Suspend> {
    cantrip_mechanics::cumulative_upkeep::resolve(g, item, cantrip_mechanics::cumulative_upkeep::Payment::PutPlusOnePlusOneCounterOnAnOpposingCreature)
}
static ANCIENT: CardDef = CardDef { name: "Sheltering Ancient cost source", types: TypeSet::of(&[CardType::Creature]), power: Some(5), toughness: Some(5),
    abilities: &[triggered(when(EventMask::ZONE_CHANGE, never), ancient_cost, "Cumulative upkeep—Put a +1/+1 counter on a creature an opponent controls.")], ..CardDef::BLANK };
fn first_counter(q: &dyn Query, _: &Asker, proposal: &Proposal) -> bool {
    matches!(proposal, Proposal::Counters(c) if c.counts[CounterKind::PlusOne as usize] > 0 && q.obj(c.object).counters(CounterKind::PlusOne) == 0)
}
fn one_more(_: &dyn Query, _: &Asker, proposal: &mut Proposal) {
    if let Proposal::Counters(c) = proposal { c.counts[CounterKind::PlusOne as usize] += 1; }
}
static FIRST: CardDef = CardDef { name: "First counter bonus", types: TypeSet::of(&[CardType::Enchantment]),
    replacements: &[replacement(ReplacementKind::Counters, first_counter, one_more, "If it has no +1/+1 counters, one more instead.")], ..CardDef::BLANK };
static MANA: CardDef = CardDef { name: "Counter cost mana source", types: TypeSet::of(&[CardType::Land]),
    abilities: &[mana_ability(TAP, add(Mana::G), "{T}: Add {G}.")], ..CardDef::BLANK };

fn put(g: &mut Game, def: &'static CardDef, player: PlayerId) -> ObjectId {
    let id = g.state.push_obj(Obj::new(g.cards.id_of(def), player, Zone::BATTLEFIELD));
    g.state.zone_push(Zone::BATTLEFIELD, id); g.state.see_statics(); id
}
fn board() -> (Game, ObjectId, ObjectId) {
    board_with(&CHANT)
}
fn board_with(def: &'static CardDef) -> (Game, ObjectId, ObjectId) {
    let cards = Box::leak(Box::new(CardTable::new([&CHANT, &CREATURE, &SEASON, &VORINCLEX, &BAN, &ANCIENT, &FIRST, &MANA])));
    let mut g = Game::new(cards, &Setup { decks: vec![vec![], vec![]], ..Default::default() });
    while !g.state.agenda.is_empty() { g.state.frame_remove(0); }
    g.state.update_turn(|t| { t.active = YOU; t.priority = YOU; t.set_step(Step::PrecombatMain); });
    let source = put(&mut g, def, YOU); let creature = put(&mut g, &CREATURE, PAYER);
    put(&mut g, &SEASON, PAYER);
    let item = crate::actions::stack_item(StackKind::Triggered, source, AbilityRef { card: cards.id_of(def), index: 0 }, YOU, &[]);
    g.state.stack_push(item); let serial = g.state.stack.last().unwrap().serial;
    g.state.frame_push(Frame::new(FrameKind::Resolve { serial }));
    (g, source, creature)
}
fn cx(g: &mut Game) -> Cx<'_> {
    Cx { g, frame: 0, cursor: 0, mana_source: None, entering: None, lineage: Vec::new(), damage: None, asked_at: None, doer: None, producing: None }
}
fn payment(g: &mut Game, source: ObjectId, subject: ObjectId) -> Request {
    assert!(cantrip_api::Game::may_pay(&mut cx(g), PAYER, source, subject, put_counters(CounterKind::MinusOne, 1), &ASK).is_err());
    g.state.agenda[0].pending.clone().unwrap()
}
fn pay(g: &mut Game, ask: &Request, source: ObjectId, subject: ObjectId) {
    let RequestKind::UnlessPays { options, .. } = &ask.kind else { panic!("payment question") };
    let answer = Answer::Act(Action::of(options.iter().find(|o| o.kind == ActionKind::PayCost).unwrap()));
    crate::check_answer(g, ask, &answer).unwrap();
    g.state.update_frame(0, |f| { f.answers.push((Some(PAYER), answer)); f.pending = None; });
    assert_eq!(cantrip_api::Game::may_pay(&mut cx(g), PAYER, source, subject, put_counters(CounterKind::MinusOne, 1), &ASK), Ok(true));
}

#[test]
fn ordinary_effect_counter_placement_is_a_doubling_season_positive_control() {
    let (mut g, _, creature) = board();
    cantrip_api::Game::add_counters_by(&mut cx(&mut g), PAYER, creature, CounterKind::MinusOne, 1);
    assert_eq!(g.obj(creature).counters(CounterKind::MinusOne), 2);
}
#[test]
fn rebinding_to_the_creature_pays_as_cost_and_keeps_original_history_cause() {
    let (mut g, chant, creature) = board();
    let ask = payment(&mut g, creature, creature);
    pay(&mut g, &ask, creature, creature);
    assert_eq!(g.obj(creature).counters(CounterKind::MinusOne), 1);
    assert_eq!(g.obj(chant).counters(CounterKind::MinusOne), 0);
    let record = g.log.iter().find(|r| matches!(r.event, Event::CountersPut { object, by: PAYER, amount: 1, .. } if object == creature)).unwrap();
    assert!(matches!(record.cause, cantrip_api::history::Cause::Ability { source, .. } if source == chant));
}
#[test]
fn a_fixed_recipient_cost_keeps_the_original_chant_in_payment_metadata() {
    let (mut g, chant, creature) = board();
    // Card/shared selection retains the original source and describes the chosen cost object, not blight.
    assert!(cantrip_api::Game::ask(&mut cx(&mut g), PAYER, RequestKind::May { source: chant, purpose: Purpose::Ask(&CHOOSE_COST) }).is_err());
    let ask = g.state.agenda[0].pending.clone().unwrap();
    assert!(matches!(ask.kind, RequestKind::May { source, .. } if source == chant));
    assert!(cantrip_api::Game::pay_counter_cost(&mut cx(&mut g), PAYER, &[(creature, CounterKind::MinusOne, 1)]));
    assert_eq!(g.obj(creature).counters(CounterKind::MinusOne), 1);
}
#[test]
fn preserving_the_chant_as_source_must_not_put_the_counter_on_the_enchantment() {
    let (mut g, chant, creature) = board();
    assert!(cantrip_api::Game::pay_counter_cost(&mut cx(&mut g), PAYER, &[(creature, CounterKind::MinusOne, 1)]));
    assert_eq!(g.obj(creature).counters(CounterKind::MinusOne), 1);
    assert_eq!(g.obj(chant).counters(CounterKind::MinusOne), 0);
}

#[test]
fn a_cost_putter_is_the_payer_even_on_an_opponents_recipient() {
    let (mut g, chant, _) = board();
    let opposing = put(&mut g, &CREATURE, YOU); put(&mut g, &VORINCLEX, PAYER);
    assert!(cantrip_api::Game::pay_counter_cost(&mut cx(&mut g), PAYER, &[(opposing, CounterKind::PlusOne, 1)]));
    assert_eq!(g.obj(opposing).counters(CounterKind::PlusOne), 2);
    assert!(g.log.iter().any(|r| matches!(r.event, Event::CountersPut { object, by: PAYER, amount: 2, .. } if object == opposing)
        && matches!(r.cause, cantrip_api::history::Cause::Ability { source, .. } if source == chant)));
}
#[test]
fn a_replacement_to_zero_still_pays_the_cost() {
    let (mut g, _, creature) = board(); put(&mut g, &VORINCLEX, YOU);
    let events = g.log.len();
    assert!(cantrip_api::Game::pay_counter_cost(&mut cx(&mut g), PAYER, &[(creature, CounterKind::MinusOne, 1)]));
    assert_eq!(g.obj(creature).counters(CounterKind::MinusOne), 0);
    assert_eq!(g.log.len(), events, "zero counters does not invent a placement event");
}
#[test]
fn cumulative_upkeep_components_on_one_creature_are_replaced_separately() {
    let (mut g, _, creature) = board(); put(&mut g, &VORINCLEX, YOU);
    assert!(cantrip_api::Game::pay_counter_cost(&mut cx(&mut g), PAYER,
        &[(creature, CounterKind::PlusOne, 1), (creature, CounterKind::PlusOne, 1)]));
    assert_eq!(g.obj(creature).counters(CounterKind::PlusOne), 0, "each one-counter cost component is halved to zero");
}
#[test]
fn one_forbidden_recipient_refuses_the_whole_payment_before_any_counters_change() {
    let (mut g, _, forbidden) = board(); put(&mut g, &BAN, PAYER);
    let allowed = put(&mut g, &CREATURE, YOU); let events = g.log.len();
    assert!(!cantrip_api::Game::pay_counter_cost(&mut cx(&mut g), PAYER,
        &[(allowed, CounterKind::MinusOne, 1), (forbidden, CounterKind::MinusOne, 1)]));
    assert_eq!(g.obj(allowed).counters(CounterKind::MinusOne), 0);
    assert_eq!(g.obj(forbidden).counters(CounterKind::MinusOne), 0);
    assert_eq!(g.log.len(), events);
}
#[test]
fn negative_and_stale_or_phased_recipients_cannot_pay_a_positive_cost() {
    let (mut g, _, creature) = board();
    assert!(!cantrip_api::Game::pay_counter_cost(&mut cx(&mut g), PAYER, &[(creature, CounterKind::MinusOne, -1)]));
    crate::phasing::phase_out(&mut cx(&mut g), &[creature], None, false);
    assert!(!cantrip_api::Game::pay_counter_cost(&mut cx(&mut g), PAYER, &[(creature, CounterKind::MinusOne, 1)]));
    let other = put(&mut g, &CREATURE, PAYER);
    cantrip_api::Game::sacrifice(&mut cx(&mut g), PAYER, &[other]);
    assert!(!cantrip_api::Game::pay_counter_cost(&mut cx(&mut g), PAYER, &[(other, CounterKind::MinusOne, 1)]));
}
#[test]
fn zero_and_empty_costs_succeed_without_counter_events() {
    let (mut g, _, creature) = board(); put(&mut g, &BAN, PAYER); let events = g.log.len();
    assert!(cantrip_api::Game::pay_counter_cost(&mut cx(&mut g), PAYER, &[(creature, CounterKind::MinusOne, 0)]));
    assert!(cantrip_api::Game::pay_counter_cost(&mut cx(&mut g), PAYER, &[]));
    assert_eq!(g.log.len(), events);
}
#[test]
fn a_native_resolution_selects_and_pays_with_original_source_metadata_and_undo_redo() {
    let (mut g, chant, creature) = board(); g.run();
    let may = g.pending().cloned().unwrap();
    assert!(matches!(may.kind, RequestKind::May { source, purpose: Purpose::Ask(_), .. } if source == chant));
    g.answer(may.id, PAYER, Answer::Yes).unwrap();
    let choose = g.pending().cloned().unwrap(); let before = g.at;
    assert!(matches!(choose.kind, RequestKind::ChooseCards { source, purpose: Purpose::PutCounters(CounterKind::MinusOne, 1), .. } if source == chant));
    g.answer(choose.id, PAYER, Answer::Cards(vec![creature])).unwrap(); let after = g.at;
    assert_eq!(g.obj(creature).counters(CounterKind::MinusOne), 1);
    assert!(g.obj(creature).became.is_none(), "Ornithopter survives one counter under Doubling Season");
    assert_eq!(g.state.player(PAYER).life, 20);
    g.goto(before); assert_eq!(g.obj(creature).counters(CounterKind::MinusOne), 0);
    assert!(matches!(g.pending().unwrap().kind, RequestKind::ChooseCards { purpose: Purpose::PutCounters(CounterKind::MinusOne, 1), .. }));
    g.goto(after); assert_eq!(g.obj(creature).counters(CounterKind::MinusOne), 1);
    assert_eq!(g.state.player(PAYER).life, 20);
}
#[test]
fn declining_a_native_resolution_preserves_the_creature_and_deals_the_damage() {
    let (mut g, _, creature) = board(); g.run(); let ask = g.pending().cloned().unwrap();
    g.answer(ask.id, PAYER, Answer::No).unwrap();
    assert_eq!(g.obj(creature).counters(CounterKind::MinusOne), 0);
    assert_eq!(g.state.player(PAYER).life, 17);
}
#[test]
fn the_shared_recipe_does_not_offer_an_unpayable_counter_cost() {
    let (mut g, _, creature) = board(); put(&mut g, &BAN, PAYER);
    g.run();
    assert_eq!(g.obj(creature).counters(CounterKind::MinusOne), 0);
    assert_eq!(g.state.player(PAYER).life, 17, "no May or selection is asked when every recipient forbids the counter");
    assert!(!matches!(g.pending().map(|r| &r.kind), Some(RequestKind::May { .. } | RequestKind::ChooseCards { .. })));
}
#[test]
fn separate_cost_components_observe_previously_placed_counters() {
    let (mut g, _, creature) = board(); put(&mut g, &FIRST, YOU);
    assert!(cantrip_api::Game::pay_counter_cost(&mut cx(&mut g), PAYER,
        &[(creature, CounterKind::PlusOne, 1), (creature, CounterKind::PlusOne, 1)]));
    assert_eq!(g.obj(creature).counters(CounterKind::PlusOne), 3, "the bonus applies to the first component only");
}
#[test]
fn sheltering_ancients_shared_recipe_selects_every_component_before_paying_any() {
    let (mut g, ancient, creature) = board_with(&ANCIENT);
    crate::counters::put(&mut cx(&mut g), ancient, CounterKind::Age, 1, YOU, crate::counters::How::TurnBased);
    g.run(); let may = g.pending().cloned().unwrap();
    assert!(matches!(may.kind, RequestKind::May { source, .. } if source == ancient));
    g.answer(may.id, YOU, Answer::Yes).unwrap();
    for _ in 0..2 {
        let ask = g.pending().cloned().unwrap();
        assert!(matches!(ask.kind, RequestKind::ChooseCards { source, purpose: Purpose::PutCounters(CounterKind::PlusOne, 1), .. } if source == ancient));
        assert_eq!(g.obj(creature).counters(CounterKind::PlusOne), 0, "all choices precede payment (CR 702.24a)");
        g.answer(ask.id, YOU, Answer::Cards(vec![creature])).unwrap();
    }
    assert_eq!(g.obj(ancient).counters(CounterKind::Age), 2);
    assert_eq!(g.obj(creature).counters(CounterKind::PlusOne), 2, "Doubling Season does not double either cost payment");
    assert!(g.log.iter().filter(|r| matches!(r.event, Event::CountersPut { object, kind: CounterKind::PlusOne, .. } if object == creature))
        .all(|r| matches!(r.event, Event::CountersPut { by: YOU, amount: 1, .. })));
}
#[test]
fn a_nonmana_counter_cost_opens_no_mana_ability_window() {
    let (mut g, _, creature) = board(); let mana_source = put(&mut g, &MANA, PAYER); g.run();
    let activate = Answer::Act(Action::of(&ActionOption::on(ActionKind::ActivateAbility, mana_source, 0)));
    let may = g.pending().cloned().unwrap();
    assert!(matches!(may.kind, RequestKind::May { .. }));
    assert!(crate::check_answer(&g, &may, &activate).is_err());
    g.answer(may.id, PAYER, Answer::Yes).unwrap();
    let choose = g.pending().cloned().unwrap();
    assert!(matches!(choose.kind, RequestKind::ChooseCards { .. }));
    assert!(crate::check_answer(&g, &choose, &activate).is_err());
    g.answer(choose.id, PAYER, Answer::Cards(vec![creature])).unwrap();
    assert!(!g.obj(mana_source).tapped);
    assert_eq!(g.state.player(PAYER).pool.get(Mana::G), 0);
}
#[test]
fn a_player_who_has_left_cannot_pay_a_counter_cost_on_another_players_object() {
    let (mut g, chant, _) = board(); g.concede(PAYER);
    assert!(!cantrip_api::Game::pay_counter_cost(&mut cx(&mut g), PAYER, &[(chant, CounterKind::PlusOne, 1)]));
    assert_eq!(g.obj(chant).counters(CounterKind::PlusOne), 0);
}
