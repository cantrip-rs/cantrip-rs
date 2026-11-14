//! Life exchanges use the bounded life-change domain without overflowing (RX-5, CR 701.12c).
use std::sync::OnceLock;
use cantrip_engine::*;
use cantrip_script::{GameScript, run_on};

fn exchange_extremes(g: &mut dyn cantrip_api::Game, item: &StackItem) -> Result<(), Suspend> {
    g.set_life_total(item.controller, -1);
    g.end_sentence();
    g.set_life_total(PlayerId(1), i32::MAX);
    g.end_sentence();
    assert_eq!(g.state().player(item.controller).life, -1);
    assert_eq!(g.state().player(PlayerId(1)).life, i32::MAX);
    cantrip_mechanics::exchange::life_totals(g, item.controller, PlayerId(1));
    assert_eq!(g.state().player(item.controller).life, i32::MAX);
    assert_eq!(g.state().player(PlayerId(1)).life, -1);
    assert_eq!(g.history().life_gained(item.controller), 2_147_483_648);
    assert_eq!(g.history().life_lost(PlayerId(1)), 2_147_483_648);
    Ok(())
}
static EXCHANGE: CardDef = CardDef {
    name: "Bounded exchange probe", oracle: true, has_cost: true, cost: ManaCost::ZERO,
    types: TypeSet::of(&[CardType::Sorcery]), spell: spell(&[], exchange_extremes), ..CardDef::BLANK
};

#[test]
fn opposite_signed_totals_do_not_overflow_the_shared_exchange() {
    static TABLE: OnceLock<CardTable> = OnceLock::new();
    let table = TABLE.get_or_init(|| CardTable::new(cantrip_cards::all().into_iter().chain([&EXCHANGE])));
    let script = GameScript::parse(r#"
[Bot1 Deck]
Bounded exchange probe
Island x20
[Bot2 Deck]
Island x20
[Actions]
Bot1.Hand.Bounded exchange probe.Cast
Bot1.Pass
Bot2.Pass
[Expect]
Bot1.Life = 2147483647
Bot2.Life = -1
"#, "bounded-life-exchange").unwrap();
    let result = run_on(&script, table).unwrap_or_else(|f| panic!("{}", f.report));
    let gain = result.game.log.iter().rev().find_map(|r| match r.event {
        Event::LifeChange { player: PlayerId(0), magnitude, negative } => Some((magnitude, negative)),
        _ => None,
    }).expect("the exchange announces its gain");
    assert_eq!(gain, (2_147_483_648, false));
}

fn halve_gain_applies(_: &dyn Query, asker: &Asker, p: &Proposal) -> bool {
    matches!(p, Proposal::LifeChange(l) if l.player == asker.controller && !l.negative && l.magnitude > 0)
}
fn halve_gain(_: &dyn Query, _: &Asker, p: &mut Proposal) {
    if let Proposal::LifeChange(l) = p { l.magnitude /= 2; }
}
static HALVER: CardDef = CardDef {
    name: "Bounded exchange halver", oracle: true, has_cost: true, cost: ManaCost::ZERO,
    types: TypeSet::of(&[CardType::Enchantment]),
    replacements: &[replacement(ReplacementKind::LifeChange, halve_gain_applies, halve_gain, "gain half that life")],
    ..CardDef::BLANK
};
fn exchange_extremes_halved(g: &mut dyn cantrip_api::Game, item: &StackItem) -> Result<(), Suspend> {
    g.set_life_total(item.controller, -1);
    g.end_sentence();
    g.set_life_total(PlayerId(1), i32::MAX);
    g.end_sentence();
    cantrip_mechanics::exchange::life_totals(g, item.controller, PlayerId(1));
    Ok(())
}
static HALVED_EXCHANGE: CardDef = CardDef {
    name: "Bounded exchange halved", oracle: true, has_cost: true, cost: ManaCost::ZERO,
    types: TypeSet::of(&[CardType::Sorcery]), spell: spell(&[], exchange_extremes_halved), ..CardDef::BLANK
};
#[test]
fn replacement_sees_the_full_gain_before_the_simultaneous_exchange() {
    static TABLE: OnceLock<CardTable> = OnceLock::new();
    let table = TABLE.get_or_init(|| CardTable::new(cantrip_cards::all().into_iter().chain([&HALVER, &HALVED_EXCHANGE])));
    let script = GameScript::parse(r#"
[Bot1 Deck]
Bounded exchange halver
Bounded exchange halved
Island x20
[Bot2 Deck]
Island x20
[Actions]
Bot1.Hand.Bounded exchange halver.Cast
Bot1.Pass
Bot2.Pass
Bot1.Hand.Bounded exchange halved.Cast
Bot1.Pass
Bot2.Pass
[Expect]
Bot1.Life = 1073741823
Bot2.Life = -1
"#, "bounded-life-exchange-replaced").unwrap();
    let result = run_on(&script, table).unwrap_or_else(|f| panic!("{}", f.report));
    assert!(result.game.log.iter().any(|r| matches!(r.event, Event::LifeChange {
        player: PlayerId(0), magnitude: 1_073_741_824, negative: false
    })));
}

fn exchange_full_span(g: &mut dyn cantrip_api::Game, item: &StackItem) -> Result<(), Suspend> {
    g.set_life_total(item.controller, i32::MIN);
    g.end_sentence();
    g.set_life_total(PlayerId(1), i32::MAX);
    g.end_sentence();
    cantrip_mechanics::exchange::life_totals(g, item.controller, PlayerId(1));
    Ok(())
}
static FULL_SPAN: CardDef = CardDef {
    name: "Full-span exchange probe", oracle: true, has_cost: true, cost: ManaCost::ZERO,
    types: TypeSet::of(&[CardType::Sorcery]), spell: spell(&[], exchange_full_span), ..CardDef::BLANK
};
#[test]
fn exchange_spans_both_i32_endpoints_as_one_gain_and_one_loss() {
    static TABLE: OnceLock<CardTable> = OnceLock::new();
    let table = TABLE.get_or_init(|| CardTable::new(cantrip_cards::all().into_iter().chain([&FULL_SPAN])));
    let script = GameScript::parse(r#"
[Bot1 Deck]
Full-span exchange probe
Island x20
[Bot2 Deck]
Island x20
[Actions]
Bot1.Hand.Full-span exchange probe.Cast
Bot1.Pass
Bot2.Pass
[Expect]
Bot1.Life = 2147483647
Bot2.Life = -2147483648
"#, "full-span-life-exchange").unwrap();
    let result = run_on(&script, table).unwrap_or_else(|f| panic!("{}", f.report));
    for (player, negative) in [(PlayerId(0), false), (PlayerId(1), true)] {
        assert_eq!(result.game.log.iter().filter(|r| matches!(r.event, Event::LifeChange {
            player: p, magnitude: u32::MAX, negative: n
        } if p == player && n == negative)).count(), 1);
    }
    let mut game = result.game;
    let end = game.at;
    let root = game.tree.root();
    game.goto(root);
    game.goto(end);
    assert_eq!(game.state.player(PlayerId(0)).life, i32::MAX);
    assert_eq!(game.state.player(PlayerId(1)).life, i32::MIN);
    for (player, negative) in [(PlayerId(0), false), (PlayerId(1), true)] {
        assert_eq!(game.log.iter().filter(|r| matches!(r.event, Event::LifeChange {
            player: p, magnitude: u32::MAX, negative: n
        } if p == player && n == negative)).count(), 1);
    }
}

fn exchange_full_span_lost(g: &mut dyn cantrip_api::Game, item: &StackItem) -> Result<(), Suspend> {
    g.set_life_total(item.controller, i32::MAX);
    g.end_sentence();
    g.set_life_total(PlayerId(1), i32::MIN);
    g.end_sentence();
    let lost = cantrip_mechanics::exchange::life_totals(g, item.controller, PlayerId(1));
    assert_eq!(lost, u32::MAX, "Mister Negative reads the full life lost this way");
    Ok(())
}
static FULL_SPAN_LOST: CardDef = CardDef {
    name: "Full-span exchange lost", oracle: true, has_cost: true, cost: ManaCost::ZERO,
    types: TypeSet::of(&[CardType::Sorcery]), spell: spell(&[], exchange_full_span_lost), ..CardDef::BLANK
};
#[test]
fn exchange_reports_full_span_lost_to_its_caller() {
    static TABLE: OnceLock<CardTable> = OnceLock::new();
    let table = TABLE.get_or_init(|| CardTable::new(cantrip_cards::all().into_iter().chain([&FULL_SPAN_LOST])));
    let script = GameScript::parse(r#"
[Bot1 Deck]
Full-span exchange lost
Island x20
[Bot2 Deck]
Island x20
[Actions]
Bot1.Hand.Full-span exchange lost.Cast
Bot1.Pass
Bot2.Pass
[Expect]
Bot1.Life = -2147483648
Bot2.Life = 2147483647
"#, "full-span-life-lost").unwrap();
    let result = run_on(&script, table);
    assert!(result.is_ok(), "{}", result.err().map(|f| f.report).unwrap_or_default());
}

fn amplify_opponent_loss(q: &dyn Query, a: &Asker, p: &Proposal) -> bool {
    matches!(p, Proposal::LifeChange(l) if l.negative && l.magnitude > 0 && q.state().opponents(a.controller).any(|x| x == l.player))
}
fn make_large_loss(_: &dyn Query, _: &Asker, p: &mut Proposal) {
    if let Proposal::LifeChange(l) = p { l.magnitude = 2_147_483_648; }
}
static AMPLIFIER: CardDef = CardDef {
    name: "Life loss amplifier probe", oracle: true, has_cost: true, cost: ManaCost::ZERO,
    types: TypeSet::of(&[CardType::Enchantment]),
    replacements: &[replacement(ReplacementKind::LifeChange, amplify_opponent_loss, make_large_loss, "lose 2147483648 life")],
    ..CardDef::BLANK
};
fn drain_large_loss(g: &mut dyn cantrip_api::Game, item: &StackItem) -> Result<(), Suspend> {
    g.set_life_total(item.controller, -1);
    g.end_sentence();
    g.set_life_total(PlayerId(1), i32::MAX);
    g.end_sentence();
    let lost = g.each_opponent_loses_life(item.controller, 1);
    assert_eq!(lost, 2_147_483_648);
    g.life_changes_together(&[LifeChange { player: item.controller, magnitude: lost, negative: false }]);
    Ok(())
}
static DRAIN: CardDef = CardDef {
    name: "Life loss drain probe", oracle: true, has_cost: true, cost: ManaCost::ZERO,
    types: TypeSet::of(&[CardType::Sorcery]), spell: spell(&[], drain_large_loss), ..CardDef::BLANK
};
#[test]
fn loss_receipt_stays_unsigned_for_extort_style_gain() {
    static TABLE: OnceLock<CardTable> = OnceLock::new();
    let table = TABLE.get_or_init(|| CardTable::new(cantrip_cards::all().into_iter().chain([&AMPLIFIER, &DRAIN])));
    let script = GameScript::parse(r#"
[Bot1 Deck]
Life loss amplifier probe
Life loss drain probe
Island x20
[Bot2 Deck]
Island x20
[Actions]
Bot1.Hand.Life loss amplifier probe.Cast
Bot1.Pass
Bot2.Pass
Bot1.Hand.Life loss drain probe.Cast
Bot1.Pass
Bot2.Pass
[Expect]
Bot1.Life = 2147483647
Bot2.Life = -1
"#, "full-width-loss-receipt").unwrap();
    let result = run_on(&script, table);
    assert!(result.is_ok(), "{}", result.err().map(|f| f.report).unwrap_or_default());
}

fn drain_multiple_large_losses(g: &mut dyn cantrip_api::Game, item: &StackItem) -> Result<(), Suspend> {
    g.set_life_total(item.controller, i32::MIN);
    g.end_sentence();
    for p in [PlayerId(1), PlayerId(2), PlayerId(3)] {
        g.set_life_total(p, i32::MAX);
        g.end_sentence();
    }
    let lost = g.each_opponent_loses_life(item.controller, 1);
    assert_eq!(lost, 3 * 2_147_483_648_u64);
    g.life_changes_together(&[LifeChange { player: item.controller, magnitude: lost, negative: false }]);
    Ok(())
}
static MULTI_DRAIN: CardDef = CardDef {
    name: "Multiple life loss probe", oracle: true, has_cost: true, cost: ManaCost::ZERO,
    types: TypeSet::of(&[CardType::Sorcery]), spell: spell(&[], drain_multiple_large_losses), ..CardDef::BLANK
};
#[test]
fn aggregate_above_u32_survives_until_a_replacement_reduces_it() {
    static TABLE: OnceLock<CardTable> = OnceLock::new();
    let table = TABLE.get_or_init(|| CardTable::new(cantrip_cards::all().into_iter().chain([&AMPLIFIER, &HALVER, &MULTI_DRAIN])));
    let script = GameScript::parse(r#"
[Bot1 Deck]
Life loss amplifier probe
Bounded exchange halver
Multiple life loss probe
Island x20
[Bot2 Deck]
Island x20
[Bot3 Deck]
Island x20
[Bot4 Deck]
Island x20
[Actions]
Bot1.Hand.Life loss amplifier probe.Cast
Bot1.Pass
Bot2.Pass
Bot3.Pass
Bot4.Pass
Bot1.Hand.Bounded exchange halver.Cast
Bot1.Pass
Bot2.Pass
Bot3.Pass
Bot4.Pass
Bot1.Hand.Multiple life loss probe.Cast
Bot1.Pass
Bot2.Pass
Bot3.Pass
Bot4.Pass
[Expect]
Bot1.Life = 1073741824
Bot2.Life = -1
Bot3.Life = -1
Bot4.Life = -1
"#, "multi-player-life-receipt").unwrap();
    let result = run_on(&script, table);
    assert!(result.is_ok(), "{}", result.err().map(|f| f.report).unwrap_or_default());
}

/// Repeated legal exchanges keep both life totals in i32, while this turn's cumulative gain and loss exceed u32.
fn repeated_exchange_history(g: &mut dyn cantrip_api::Game, item: &StackItem) -> Result<(), Suspend> {
    let a = item.controller;
    let b = PlayerId(1);
    g.set_life_total(a, i32::MIN);
    g.end_sentence();
    g.set_life_total(b, i32::MAX);
    g.end_sentence();
    let before_gain = g.history().life_gained(a) as u64;
    let before_loss = g.history().life_lost(a) as u64;
    for _ in 0..4 {
        cantrip_mechanics::exchange::life_totals(g, a, b);
        g.end_sentence();
    }
    assert_eq!(g.history().life_gained(a) as u64, before_gain + 2 * u64::from(u32::MAX));
    assert_eq!(g.history().life_lost(a) as u64, before_loss + 2 * u64::from(u32::MAX));
    // Archfiend/Gul's actual shared recipe must pass the whole opponent history to a replacement.
    let lost = cantrip_mechanics::effects::each_opponent_loses_life_lost_this_turn(g, a);
    assert_eq!(lost, 1);
    assert_eq!(g.state().player(b).life, i32::MAX - 1);
    Ok(())
}
fn large_historical_loss(_: &dyn Query, _: &Asker, p: &Proposal) -> bool {
    matches!(p, Proposal::LifeChange(l) if l.negative && l.magnitude > u64::from(u32::MAX))
}
fn reduce_historical_loss(_: &dyn Query, _: &Asker, p: &mut Proposal) {
    if let Proposal::LifeChange(l) = p {
        assert_eq!(l.magnitude, 2 * u64::from(u32::MAX));
        l.magnitude = 1;
    }
}
static HISTORY_REDUCER: CardDef = CardDef {
    name: "Historical life reduction probe", oracle: true, has_cost: true, cost: ManaCost::ZERO,
    types: TypeSet::of(&[CardType::Enchantment]),
    replacements: &[replacement(ReplacementKind::LifeChange, large_historical_loss, reduce_historical_loss, "reduce the historical loss")],
    ..CardDef::BLANK
};
static REPEATED_HISTORY: CardDef = CardDef {
    name: "Repeated life exchange history probe", oracle: true, has_cost: true, cost: ManaCost::ZERO,
    types: TypeSet::of(&[CardType::Sorcery]), spell: spell(&[], repeated_exchange_history), ..CardDef::BLANK
};
#[test]
fn repeated_exchange_history_sums_without_u32_overflow() {
    static TABLE: OnceLock<CardTable> = OnceLock::new();
    let table = TABLE.get_or_init(|| CardTable::new(cantrip_cards::all().into_iter().chain([&REPEATED_HISTORY, &HISTORY_REDUCER])));
    let script = GameScript::parse(r#"
[Bot1 Deck]
Historical life reduction probe
Repeated life exchange history probe
Island x20
[Bot2 Deck]
Island x20
[Actions]
Bot1.Hand.Historical life reduction probe.Cast
Bot1.Pass
Bot2.Pass
Bot1.Hand.Repeated life exchange history probe.Cast
Bot1.Pass
Bot2.Pass
[Expect]
Bot1.Life = -2147483648
Bot2.Life = 2147483646
"#, "repeated-exchange-history").unwrap();
    run_on(&script, table).unwrap_or_else(|f| panic!("{}", f.report));
}
