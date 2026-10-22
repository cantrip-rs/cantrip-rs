//! CR 304.5/602.5e: explicit instant-only mana abilities require priority, not a mana-payment window.

use super::*;
use crate::Game as RulesGame;
use cantrip_state::{Frame, FrameKind};
use std::sync::OnceLock;

const YOU: PlayerId = PlayerId(0);

fn nothing(_: &mut dyn Game, _: &StackItem) -> Result<(), Suspend> {
    Ok(())
}

static PRIORITY_MANA: CardDef = CardDef {
    name: "Priority-only mana probe",
    types: TypeSet::of(&[CardType::Artifact]),
    abilities: &[mana_ability(TAP, add_n(Mana::U, 3), "{T}: Add {U}{U}{U}. Activate only as an instant.").only_as_an_instant()],
    ..CardDef::BLANK
};
static ORDINARY_MANA: CardDef = CardDef {
    name: "Ordinary mana probe",
    abilities: &[mana_ability(TAP, add(Mana::C), "{T}: Add {C}.")],
    ..PRIORITY_MANA
};
static HAND_MANA: CardDef = CardDef {
    name: "Priority-only hand mana probe",
    abilities: &[mana_ability(EXILE_THIS, add(Mana::U), "Exile this card: Add {U}. Activate only as an instant.").from(ZoneKind::Hand).only_as_an_instant()],
    ..PRIORITY_MANA
};
static PLAYER_MANA: CardDef = CardDef {
    name: "Priority-only player mana probe",
    delayed: &[mana_ability(Cost { life: 1, ..Cost::NONE }, add(Mana::U), "Pay 1 life: Add {U}. Activate only as an instant.").only_as_an_instant()],
    ..CardDef::BLANK
};
static SPELL: CardDef = CardDef {
    name: "Priority timing spell probe",
    types: TypeSet::of(&[CardType::Instant]),
    has_cost: true,
    cost: mana("{U}"),
    spell: spell(&[], nothing),
    ..CardDef::BLANK
};
static PAID_ABILITY: CardDef = CardDef {
    name: "Priority timing ability probe",
    abilities: &[ability(cost("{U}"), nothing, "{U}: Nothing.")],
    ..PRIORITY_MANA
};
static FILTER: CardDef = CardDef {
    name: "Priority timing filter probe",
    abilities: &[mana_ability(cost("{U}, {T}"), add_n(Mana::B, 2), "{U}, {T}: Add {B}{B}.")],
    ..PRIORITY_MANA
};
fn tapped_for_mana(_: &dyn Query, _: ObjectId, event: &Event) -> Option<Fired> {
    match event {
        Event::TappedForMana { player, .. } => Fired::about(Target::Player(*player)),
        _ => None,
    }
}
static MANA_TRIGGER: CardDef = CardDef {
    name: "Priority-only mana trigger probe",
    types: TypeSet::of(&[CardType::Enchantment]),
    abilities: &[triggered(when(EventMask::TAPPED, tapped_for_mana), |g, item| {
        g.add_mana(item.subject.player().unwrap(), Mana::U, 1);
        Ok(())
    }, "Whenever a player taps a permanent for mana, that player adds an additional {U}.").mana_ability(TriggeredMana::One(Mana::U))],
    ..CardDef::BLANK
};

fn board() -> (RulesGame, ObjectId, ObjectId, ObjectId) {
    static TABLE: OnceLock<CardTable> = OnceLock::new();
    let cards = TABLE.get_or_init(|| CardTable::new([&PRIORITY_MANA, &ORDINARY_MANA, &HAND_MANA, &PLAYER_MANA, &SPELL, &PAID_ABILITY, &FILTER, &MANA_TRIGGER]));
    let mut game = RulesGame::new(cards, &crate::Setup { decks: vec![Vec::new(), Vec::new()], ..Default::default() });
    game.state.update_turn(|t| {
        t.number = 1;
        t.active = YOU;
        t.priority = YOU;
        t.set_step(Step::PrecombatMain);
    });
    let mut put = |def, zone| {
        let id = game.state.push_obj(Obj::new(cards.id_of(def), YOU, zone));
        game.state.zone_push(zone, id);
        id
    };
    let restricted = put(&PRIORITY_MANA, Zone::BATTLEFIELD);
    let ordinary = put(&ORDINARY_MANA, Zone::BATTLEFIELD);
    let spell = put(&SPELL, Zone::hand(YOU));
    (game, restricted, ordinary, spell)
}

fn cx(game: &mut RulesGame) -> Cx<'_> {
    Cx { g: game, frame: usize::MAX, cursor: 0, mana_source: None, entering: None, lineage: Vec::new(), asked_at: None, doer: None }
}

#[test]
fn all_mana_sources_require_priority_if_their_own_ability_says_so() {
    let (mut game, restricted, ordinary, _) = board();
    let hand = game.state.push_obj(Obj::new(game.cards.id_of(&HAND_MANA), YOU, Zone::hand(YOU)));
    game.state.zone_push(Zone::hand(YOU), hand);
    game.state.effect_push(ContinuousEffect {
        source: restricted, affected: restricted, controller: YOU,
        kind: EffectKind::PlayerManaAbility(AbilityRef::delayed(game.cards.id_of(&PLAYER_MANA), 0)),
        duration: Duration::EndOfTurn, timestamp: 1,
    });
    let cx = cx(&mut game);
    let mut priority = Vec::new();
    mana_ability_options(&cx, YOU, ManaWindow::Priority, &mut priority);
    assert_eq!(priority.len(), 4, "permanent, hand, player and ordinary sources are usable with priority");
    assert!(priority.iter().any(|o| o.source == Some(hand)));
    let mut payment = Vec::new();
    mana_ability_options(&cx, YOU, ManaWindow::Payment, &mut payment);
    assert_eq!(payment.iter().map(|o| o.source.unwrap()).collect::<Vec<_>>(), vec![ordinary]);
    assert_eq!(available(&cx, YOU).most_unrestricted(), 1);
    assert!(available_without(&cx, YOU, None, ManaWindow::Priority).covers(&mana("{U}{U}{U}")));
}

#[test]
fn affordability_does_not_advertise_priority_only_mana_before_it_is_floated() {
    let (mut game, restricted, _, spell) = board();
    let ability = game.state.push_obj(Obj::new(game.cards.id_of(&PAID_ABILITY), YOU, Zone::BATTLEFIELD));
    game.state.zone_push(Zone::BATTLEFIELD, ability);
    let filter = game.state.push_obj(Obj::new(game.cards.id_of(&FILTER), YOU, Zone::BATTLEFIELD));
    game.state.zone_push(Zone::BATTLEFIELD, filter);
    let mut cx = cx(&mut game);
    let offered = priority_options(&cx, YOU);
    assert!(offered.iter().any(|o| o.source == Some(restricted)), "priority control");
    assert!(!offered.iter().any(|o| o.source == Some(spell)), "spell cannot use an unactivated priority-only source");
    assert!(!offered.iter().any(|o| o.source == Some(ability)), "neither can an activated ability");
    assert!(!offered.iter().any(|o| o.source == Some(filter)), "a mana ability cannot use it to pay its own mana cost");
    assert!(!can_afford(&cx, YOU, spell, &mana("{U}")));
    activate_mana(&mut cx, YOU, restricted, 0, None, false, &[]).unwrap();
    assert!(can_afford(&cx, YOU, spell, &mana("{U}")), "mana already in pool remains spendable");
    let offered = priority_options(&cx, YOU);
    assert!(offered.iter().any(|o| o.source == Some(spell)));
    assert!(offered.iter().any(|o| o.source == Some(ability)));
    assert!(offered.iter().any(|o| o.source == Some(filter)));
}

#[test]
fn every_payment_context_uses_the_same_restricted_inventory() {
    for purpose in [Paying::Casting, Paying::Activating, Paying::Other] {
        let (mut game, restricted, ordinary, source) = board();
        game.state.frame_push(Frame::new(FrameKind::Act { player: YOU }));
        let mut cx = cx(&mut game);
        cx.frame = 0;
        let helpers = Helpers { purpose, ..Helpers::NONE };
        assert!(pay_mana_with(&mut cx, YOU, source, mana("{U}"), &helpers).is_err());
        let ask = cx.s().agenda[0].pending.as_ref().unwrap();
        let RequestKind::PayCosts { options, .. } = &ask.kind else { panic!("payment question") };
        assert!(options.iter().any(|o| o.source == Some(ordinary)));
        assert!(!options.iter().any(|o| o.source == Some(restricted)));
        let forged = Answer::Act(Action::of(&ActionOption::on(ActionKind::ActivateAbility, restricted, 0)));
        assert!(crate::check_answer(cx.g, ask, &forged).is_err(), "unoffered activation cannot be submitted");
    }
}

#[test]
fn unless_and_variable_payments_do_not_create_priority() {
    const ASK: Question = Question::new("Pay", "pay {cost}?", "", "Pay", "Decline");
    for cost in [cost("{U}"), cost("{X}{U}")] {
        let (mut game, restricted, ordinary, source) = board();
        game.state.frame_push(Frame::new(FrameKind::Resolve));
        let mut cx = cx(&mut game);
        cx.frame = 0;
        let outcome = if cost.mana.x == 0 {
            may_pay(&mut cx, YOU, source, source, &cost, &ASK).map(|_| ())
        } else {
            may_pay_x(&mut cx, YOU, source, source, &cost, &ASK).map(|_| ())
        };
        assert!(outcome.is_err());
        let ask = cx.s().agenda[0].pending.as_ref().unwrap();
        let RequestKind::UnlessPays { options, .. } = &ask.kind else { panic!("unless question") };
        assert!(options.iter().any(|o| o.source == Some(ordinary)));
        assert!(!options.iter().any(|o| o.source == Some(restricted)));
    }
}

#[test]
fn a_granted_mana_ability_keeps_its_own_priority_restriction() {
    let (mut game, restricted, ordinary, _) = board();
    let ability = AbilityRef { card: game.cards.id_of(&PRIORITY_MANA), index: 0 };
    game.state.effect_push(ContinuousEffect {
        source: restricted, affected: ordinary, controller: YOU, kind: EffectKind::AddAbility(ability),
        duration: Duration::EndOfTurn, timestamp: 1,
    });
    let cx = cx(&mut game);
    let mut priority = Vec::new();
    mana_ability_options(&cx, YOU, ManaWindow::Priority, &mut priority);
    assert_eq!(priority.iter().filter(|o| o.source == Some(ordinary)).count(), 2);
    let mut payment = Vec::new();
    mana_ability_options(&cx, YOU, ManaWindow::Payment, &mut payment);
    assert_eq!(payment.len(), 1, "ordinary ability remains usable, granted one does not");
    assert_eq!(available(&cx, YOU).most_unrestricted(), 1);
    assert!(activated(&cx, ability).unwrap().priority_only);
}

#[test]
fn a_copy_keeps_priority_only_mana_but_produces_nothing_during_payment() {
    let (mut game, restricted, ordinary, _) = board();
    let card = game.cards.id_of(&PRIORITY_MANA);
    game.state.update_obj(ordinary, |o| o.copy = Some(Copiable::of(card)));
    let cx = cx(&mut game);
    assert_eq!(priority_options(&cx, YOU).iter().filter(|o| o.source == Some(ordinary)).count(), 1);
    assert!(mana_options_while_paying(&cx, YOU, restricted, Paying::Other).is_empty());
    assert_eq!(available(&cx, YOU).most_unrestricted(), 0);
}

#[test]
fn priority_only_mana_still_resolves_immediately_with_its_triggered_mana() {
    let (mut game, restricted, ordinary, _) = board();
    let id = game.state.push_obj(Obj::new(game.cards.id_of(&MANA_TRIGGER), YOU, Zone::BATTLEFIELD));
    game.state.zone_push(Zone::BATTLEFIELD, id);
    let mut cx = cx(&mut game);
    assert_eq!(available(&cx, YOU).most_unrestricted(), 2, "ordinary tap plus triggered mana, no restricted tap yield");
    activate_mana(&mut cx, YOU, restricted, 0, None, false, &[]).unwrap();
    assert_eq!(cx.s().player(YOU).pool.get(Mana::U), 4);
    assert!(cx.s().stack.is_empty());
    assert!(!cx.s().obj(ordinary).tapped);
}
