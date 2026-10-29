//! Excess actually dealt (CR 120.10): Violent Echoes, Toralf and Aegar share this rules information.
use crate::{Cx, Game, Setup, ops};
use cantrip_api::*;
const YOU: PlayerId = PlayerId(0);
static BEAR: CardDef =
    CardDef { name: "Excess Bear", types: TypeSet::of(&[CardType::Creature]), power: Some(2), toughness: Some(4), ..CardDef::BLANK };
static TOUCH: CardDef = CardDef { name: "Excess Touch", keywords: Keywords::of(&[Keyword::Deathtouch]), ..BEAR };
static INFECT: CardDef = CardDef { name: "Excess Infect", keywords: Keywords::of(&[Keyword::Infect]), ..BEAR };
static WALKER: CardDef = CardDef { name: "Excess Walker", types: TypeSet::of(&[CardType::Planeswalker]), ..CardDef::BLANK };
static BATTLE: CardDef = CardDef { name: "Excess Battle", types: TypeSet::of(&[CardType::Battle]), ..CardDef::BLANK };
static MULTI: CardDef =
    CardDef { name: "Excess Multitype", types: TypeSet::of(&[CardType::Creature, CardType::Planeswalker, CardType::Battle]), ..BEAR };
fn to_self(_: &dyn Query, a: &Asker, d: &DamageEvent) -> bool {
    d.target == Target::Object(a.source)
}
fn two(_: &dyn Query, _: &Asker, _: &DamageEvent) -> u16 {
    2
}
static SHIELDED: CardDef =
    CardDef { name: "Excess Shielded", preventions: &[PreventionDef { hint: "prevent two", applies: to_self, amount: Some(two) }], ..BEAR };
static IMMUNE: CardDef =
    CardDef { name: "Excess Immune", preventions: &[PreventionDef { hint: "prevent all", applies: to_self, amount: None }], ..BEAR };
fn to_another(_: &dyn Query, a: &Asker, p: &Proposal) -> bool {
    matches!(p, Proposal::Damage(d) if matches!(d.target, Target::Object(id) if id != a.source))
}
fn redirect(_: &dyn Query, a: &Asker, p: &mut Proposal) {
    if let Proposal::Damage(d) = p {
        d.target = Target::Object(a.source);
    }
}
static REDIRECTOR: CardDef = CardDef {
    name: "Excess Redirector",
    replacements: &[replacement(ReplacementKind::Damage, to_another, redirect, "redirect to this")],
    ..BEAR
};
fn to_self_proposed(_: &dyn Query, a: &Asker, p: &Proposal) -> bool {
    matches!(p, Proposal::Damage(d) if d.target == Target::Object(a.source))
}
fn heal(_: &dyn Query, _: &Asker, p: &mut Proposal) {
    if let Proposal::Damage(d) = p {
        d.heal_marked = true;
    }
}
/// Wolverine, Fierce Fighter's "instead that damage is dealt, but all other damage already dealt to him is healed".
static HEALER: CardDef = CardDef {
    name: "Excess Healer",
    replacements: &[replacement(ReplacementKind::Damage, to_self_proposed, heal, "heal the rest")],
    ..BEAR
};
fn damage(_: &dyn Query, _: &Asker, p: &Proposal) -> bool {
    matches!(p, Proposal::Damage(_))
}
fn double(_: &dyn Query, _: &Asker, p: &mut Proposal) {
    if let Proposal::Damage(d) = p {
        d.amount *= 2;
    }
}
static DOUBLER: CardDef = CardDef {
    name: "Excess Doubler",
    types: TypeSet::of(&[CardType::Enchantment]),
    replacements: &[replacement(ReplacementKind::Damage, damage, double, "double")],
    ..CardDef::BLANK
};
fn ascendant_itself(_: &dyn LayerQuery, source: ObjectId, affected: ObjectId, _: &Chars) -> bool {
    source == affected
}
fn ascendant_toughness(q: &dyn LayerQuery, source: ObjectId, chars: &mut Chars) {
    if q.state().player(q.state().obj(source).controller).life >= 30 {
        chars.power = Some(chars.power.unwrap_or(0) + 5);
        chars.toughness = Some(chars.toughness.unwrap_or(0) + 5);
    }
}
static ASCENDANT: CardDef = CardDef {
    name: "Excess Serra Ascendant",
    power: Some(1),
    toughness: Some(1),
    statics: &[static_ability(
        Layer::PtModify,
        ascendant_itself,
        ascendant_toughness,
        "As long as you have 30 or more life, this creature gets +5/+5.",
    )],
    ..BEAR
};
static REDIRECTING_ASCENDANT: CardDef = CardDef {
    name: "Excess Redirecting Ascendant",
    replacements: &[replacement(ReplacementKind::Damage, to_another, redirect, "redirect to this")],
    ..ASCENDANT
};
fn game(defs: &[&'static CardDef]) -> (Game, Vec<ObjectId>) {
    let table: &'static CardTable = Box::leak(Box::new(CardTable::new([
        &BEAR,
        &TOUCH,
        &INFECT,
        &WALKER,
        &BATTLE,
        &MULTI,
        &SHIELDED,
        &IMMUNE,
        &REDIRECTOR,
        &DOUBLER,
        &ASCENDANT,
        &REDIRECTING_ASCENDANT,
        &HEALER,
    ])));
    let mut g = Game::new(table, &Setup { decks: vec![Vec::new(), Vec::new()], ..Default::default() });
    g.state.update_turn(|t| t.number = 1);
    let mut ids = Vec::new();
    for &def in defs {
        let id = g.state.push_obj(Obj::new(table.id_of(def), YOU, Zone::BATTLEFIELD));
        let timestamp = g.state.timestamp();
        g.state.update_obj(id, |o| o.timestamp = timestamp);
        g.state.zone_push(Zone::BATTLEFIELD, id);
        g.state.see_statics();
        ids.push(id);
    }
    (g, ids)
}
fn cx(g: &mut Game) -> Cx<'_> {
    Cx {
        g,
        frame: usize::MAX,
        cursor: 0,
        mana_source: None,
        entering: None,
        lineage: Vec::new(),
        asked_at: None,
        doer: None,
        producing: None,
    }
}
fn hit(g: &mut Game, source: ObjectId, target: ObjectId, amount: u16) -> Option<DamageResult> {
    cantrip_api::Game::damage_result(&mut cx(g), source, Target::Object(target), amount)
}
fn recorded(g: &Game) -> Vec<(Target, u16, u32)> {
    g.log
        .iter()
        .filter_map(|r| match r.event {
            Event::Damage { target, amount, recipient_excess, .. } => Some((target, amount, recipient_excess)),
            _ => None,
        })
        .collect()
}
#[test]
fn excess_damage_uses_toughness_and_preexisting_marks_and_exact_lethal() {
    for (marks, amount, excess) in [(0, 4, 0), (0, 6, 2), (2, 6, 4), (4, 1, 1)] {
        let (mut g, ids) = game(&[&BEAR, &BEAR]);
        g.state.update_obj(ids[1], |o| o.damage = marks);
        assert_eq!(hit(&mut g, ids[0], ids[1], amount), Some(DamageResult { target: Target::Object(ids[1]), amount, excess }));
        assert_eq!(recorded(&g), [(Target::Object(ids[1]), amount, excess)]);
    }
}
#[test]
fn excess_damage_uses_loyalty_defense_and_greatest_of_multiple_types() {
    for (def, loyalty, defense, excess) in [(&WALKER, 3, 0, 3), (&BATTLE, 0, 5, 1), (&MULTI, 2, 1, 5)] {
        let (mut g, ids) = game(&[&BEAR, def]);
        crate::counters::put(&mut cx(&mut g), ids[1], CounterKind::Loyalty, loyalty, YOU, crate::counters::How::Effect);
        crate::counters::put(&mut cx(&mut g), ids[1], CounterKind::Defense, defense, YOU, crate::counters::How::Effect);
        assert_eq!(hit(&mut g, ids[0], ids[1], 6).unwrap().excess, excess);
    }
}
#[test]
fn excess_damage_is_based_on_actual_prevented_and_replaced_amounts() {
    let (mut g, ids) = game(&[&BEAR, &SHIELDED]);
    assert_eq!(hit(&mut g, ids[0], ids[1], 7), Some(DamageResult { target: Target::Object(ids[1]), amount: 5, excess: 1 }));
    let (mut g, ids) = game(&[&BEAR, &IMMUNE]);
    assert_eq!(hit(&mut g, ids[0], ids[1], 7), None);
    assert!(recorded(&g).is_empty());
    let (mut g, ids) = game(&[&BEAR, &BEAR, &DOUBLER]);
    assert_eq!(hit(&mut g, ids[0], ids[1], 3), Some(DamageResult { target: Target::Object(ids[1]), amount: 6, excess: 2 }));
}
#[test]
fn excess_damage_names_the_redirected_recipient_and_players_have_none() {
    let (mut g, ids) = game(&[&BEAR, &BEAR, &REDIRECTOR]);
    assert_eq!(hit(&mut g, ids[0], ids[1], 6), Some(DamageResult { target: Target::Object(ids[2]), amount: 6, excess: 2 }));
    assert_eq!(g.state.obj(ids[1]).damage, 0);
    let (mut g, ids) = game(&[&BEAR]);
    assert_eq!(
        cantrip_api::Game::damage_result(&mut cx(&mut g), ids[0], Target::Player(PlayerId(1)), 6),
        Some(DamageResult { target: Target::Player(PlayerId(1)), amount: 6, excess: 0 })
    );
}
/// CR 120.4a: damage beyond 1 from a source with deathtouch is excess, noncombat damage as much as combat.
#[test]
fn excess_damage_noncombat_deathtouch_counts_one_as_lethal() {
    let (mut g, ids) = game(&[&TOUCH, &BEAR]);
    assert_eq!(hit(&mut g, ids[0], ids[1], 6).unwrap().excess, 5);
}
/// A creature already marked with lethal damage takes no further damage as lethal, deathtouch or not (CR 120.4a, 120.6).
#[test]
fn excess_damage_deathtouch_to_a_creature_already_marked_lethal_is_all_excess() {
    let (mut g, ids) = game(&[&TOUCH, &BEAR]);
    g.state.update_obj(ids[1], |o| o.damage = 4);
    assert_eq!(hit(&mut g, ids[0], ids[1], 6).unwrap().excess, 6);
}
#[test]
fn excess_damage_sums_a_simultaneous_event_and_preserves_predamage_loyalty() {
    for (def, excess) in [(&BEAR, 3), (&WALKER, 4)] {
        let (mut g, ids) = game(&[&BEAR, &BEAR, def]);
        crate::counters::put(&mut cx(&mut g), ids[2], CounterKind::Loyalty, 3, YOU, crate::counters::How::Effect);
        assert_eq!(
            ops::damage_together(&mut cx(&mut g), &[(ids[0], Target::Object(ids[2]), 3), (ids[1], Target::Object(ids[2]), 4)]),
            [3, 4]
        );
        assert_eq!(recorded(&g), [(Target::Object(ids[2]), 3, excess), (Target::Object(ids[2]), 4, excess)]);
    }
}
#[test]
fn excess_damage_uses_toughness_before_simultaneous_infect_counters() {
    let (mut g, ids) = game(&[&INFECT, &BEAR, &BEAR]);
    ops::damage_together(&mut cx(&mut g), &[(ids[0], Target::Object(ids[2]), 2), (ids[1], Target::Object(ids[2]), 4)]);
    assert_eq!(recorded(&g), [(Target::Object(ids[2]), 2, 2), (Target::Object(ids[2]), 4, 2)]);
}
#[test]
fn excess_damage_zero_or_a_missing_recipient_deals_nothing() {
    let (mut g, ids) = game(&[&BEAR, &BEAR]);
    assert_eq!(hit(&mut g, ids[0], ids[1], 0), None);
    g.state.zone_remove(Zone::BATTLEFIELD, ids[1]);
    g.state.update_obj(ids[1], |o| o.zone = Zone::graveyard(YOU));
    g.state.zone_push(Zone::graveyard(YOU), ids[1]);
    assert_eq!(hit(&mut g, ids[0], ids[1], 6), None);
}

/// CR 702.2c's one-damage lethal exception is for combat damage assigned.
#[test]
fn excess_damage_combat_deathtouch_counts_one_as_lethal() {
    let (mut g, ids) = game(&[&TOUCH, &BEAR]);
    assert_eq!(ops::damage_result(&mut cx(&mut g), ids[0], Target::Object(ids[1]), 6, true).unwrap().excess, 5);
}

/// Damage to an earlier recipient must not lower the pre-event toughness used by a later one (CR 120.10).
#[test]
fn excess_damage_snapshots_all_recipients_before_any_simultaneous_damage() {
    let (mut g, ids) = game(&[&BEAR, &ASCENDANT]);
    g.state.update_player(YOU, |p| p.life = 30);
    ops::damage_together(&mut cx(&mut g), &[(ids[0], Target::Player(YOU), 3), (ids[0], Target::Object(ids[1]), 3)]);
    assert_eq!(recorded(&g), [(Target::Player(YOU), 3, 0), (Target::Object(ids[1]), 3, 0)]);
}

#[test]
fn excess_damage_snapshots_redirected_recipients_before_the_simultaneous_event() {
    let (mut g, ids) = game(&[&BEAR, &BEAR, &REDIRECTING_ASCENDANT]);
    g.state.update_player(YOU, |p| p.life = 30);
    ops::damage_together(&mut cx(&mut g), &[(ids[0], Target::Player(YOU), 3), (ids[0], Target::Object(ids[1]), 3)]);
    assert_eq!(recorded(&g), [(Target::Player(YOU), 3, 0), (Target::Object(ids[2]), 3, 0)]);
}
#[test]
fn excess_damage_combat_snapshots_recipients_before_any_assignment() {
    let (mut g, ids) = game(&[&BEAR, &ASCENDANT]);
    g.state.update_player(YOU, |p| p.life = 30);
    let hits = [(ids[0], Target::Player(YOU), 3), (ids[0], Target::Object(ids[1]), 3)];
    let mut cx = cx(&mut g);
    let mut batch = ops::DamageBatch::new(&cx, &hits);
    let mut events: Vec<Event> =
        hits.into_iter().filter_map(|(s, t, n)| ops::deal_damage_with(&mut cx, s, t, n, true, &mut batch)).collect();
    batch.finish(&mut cx, &mut events);
    cx.emit_batch(&events, None);
    assert_eq!(recorded(&g), [(Target::Player(YOU), 3, 0), (Target::Object(ids[1]), 3, 0)]);
}

/// Damage healed as this damage is dealt (Wolverine, Fierce Fighter) is no longer marked: lethal is the whole toughness.
#[test]
fn excess_damage_heals_the_marks_its_replacement_heals() {
    let (mut g, ids) = game(&[&BEAR, &HEALER]);
    g.state.update_obj(ids[1], |o| o.damage = 2);
    assert_eq!(hit(&mut g, ids[0], ids[1], 6).unwrap().excess, 2);
    assert_eq!(g.state.obj(ids[1]).damage, 6);
}

/// Healing (CR 701.69a) removes marked damage and its deathtouch state.
#[test]
fn heal_removes_all_marked_damage() {
    let (mut g, ids) = game(&[&BEAR]);
    g.state.update_obj(ids[0], |o| {
        o.damage = 3;
        o.deathtouched = true;
    });
    let mut cx = cx(&mut g);
    cantrip_api::Game::heal(&mut cx, ids[0]);
    assert_eq!(g.state.obj(ids[0]).damage, 0);
    assert!(!g.state.obj(ids[0]).deathtouched);
}
