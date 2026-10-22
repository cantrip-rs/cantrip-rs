//! CR 102.1/120.1/800.4a: departed recipients cannot create damage, prevention, replacement or lifelink events.

use super::*;
use crate::Game as RulesGame;

const YOU: PlayerId = PlayerId(0);
const LEFT: PlayerId = PlayerId(1);
const OTHER: PlayerId = PlayerId(2);

static SOURCE: CardDef = CardDef {
    name: "Departed damage lifelink probe",
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(3),
    toughness: Some(3),
    keywords: Keywords::of(&[Keyword::Lifelink]),
    ..CardDef::BLANK
};

fn any_damage(_: &dyn Query, _: &Asker, proposal: &Proposal) -> bool {
    matches!(proposal, Proposal::Damage(_))
}

fn unchanged(_: &dyn Query, _: &Asker, _: &mut Proposal) {}

fn bonus(g: &mut dyn cantrip_api::Game, asker: &Asker, _: &Proposal, _: &Proposal) {
    g.gain_life(asker.controller, 7);
}

static REPLACER: CardDef = CardDef {
    name: "Departed damage replacement probe",
    types: TypeSet::of(&[CardType::Enchantment]),
    replacements: &[replacement(ReplacementKind::Damage, any_damage, unchanged, "also gain seven life").then(bonus)],
    ..CardDef::BLANK
};

fn redirectable(_: &dyn Query, _: &Asker, proposal: &Proposal) -> bool {
    matches!(proposal, Proposal::Damage(d) if d.target == Target::Player(OTHER))
}

fn to_departed(_: &dyn Query, _: &Asker, proposal: &mut Proposal) {
    if let Proposal::Damage(d) = proposal { d.target = Target::Player(LEFT); }
}

static REDIRECT: CardDef = CardDef {
    name: "Departed damage redirect probe",
    replacements: &[replacement(ReplacementKind::Damage, redirectable, to_departed, "redirect to departed player")],
    ..REPLACER
};

fn all_damage(_: &dyn Query, _: &Asker, _: ObjectId, _: &DamageEvent) -> bool {
    true
}

static SHIELD: CardDef = CardDef {
    name: "Departed damage shield probe",
    granted_preventions: &[GrantedPreventionDef { hint: "prevent the next damage", damage: "damage", of_it: None, applies: all_damage }],
    ..CardDef::BLANK
};

fn game() -> (RulesGame, ObjectId) {
    let cards = Box::leak(Box::new(CardTable::new([&SOURCE, &REPLACER, &SHIELD, &REDIRECT])));
    let mut game = RulesGame::new(cards, &crate::Setup { decks: vec![Vec::new(); 3], ..Default::default() });
    let id = game.state.push_obj(Obj::new(cards.id_of(&SOURCE), YOU, Zone::BATTLEFIELD));
    game.state.zone_push(Zone::BATTLEFIELD, id);
    game.state.update_player(LEFT, |p| p.lost = true);
    (game, id)
}

fn cx(game: &mut RulesGame) -> Cx<'_> {
    Cx { g: game, frame: usize::MAX, cursor: 0, mana_source: None, entering: None, lineage: Vec::new(), asked_at: None, doer: None }
}

#[test]
fn departed_damage_is_rejected_before_replacements_and_lifelink() {
    let (mut game, source) = game();
    let id = game.state.push_obj(Obj::new(game.cards.id_of(&REPLACER), YOU, Zone::BATTLEFIELD));
    game.state.zone_push(Zone::BATTLEFIELD, id);
    let mut cx = cx(&mut game);
    assert_eq!(damage(&mut cx, source, Target::Player(LEFT), 1, false), 0);
    assert_eq!(cx.s().player(YOU).life, 20, "no replacement bonus or lifelink");
    assert_eq!(cx.s().player(LEFT).life, 20);
    assert!(cx.g.log.is_empty(), "no damage or life event");
    assert_eq!(damage(&mut cx, source, Target::Player(OTHER), 1, false), 1);
    assert_eq!(cx.s().player(YOU).life, 28, "same replacement and lifelink apply to a live recipient");
    assert_eq!(cx.s().player(OTHER).life, 19);
}

#[test]
fn departed_damage_neither_consumes_nor_orders_prevention_shields() {
    let (mut game, source) = game();
    let card = game.cards.id_of(&SHIELD);
    game.state.effect_push(ContinuousEffect {
        source, affected: source, controller: YOU,
        kind: EffectKind::Prevention { card, index: 0, to: Target::Player(LEFT), shield: Shield::Next(3) },
        duration: Duration::EndOfTurn, timestamp: 1,
    });
    let mut cx = cx(&mut game);
    let hits = [(source, Target::Player(LEFT), 2), (source, Target::Player(LEFT), 2)];
    assert!(order_for_shields(&mut cx, &hits, false).is_empty());
    assert_eq!(damage_together(&mut cx, &hits), vec![0, 0]);
    assert!(cx.g.log.is_empty());
    assert!(matches!(cx.s().effects[0].kind, EffectKind::Prevention { shield: Shield::Next(3), .. }));
}

#[test]
fn simultaneous_damage_keeps_live_and_zero_life_recipients_and_source_lki() {
    let (mut game, source) = game();
    let mut cx = cx(&mut game);
    let gone = exile_all(&mut cx, &[source]);
    assert_eq!(gone.len(), 1);
    cx.st().update_player(OTHER, |p| p.life = 0);
    let hits = [(source, Target::Player(LEFT), 5), (source, Target::Player(OTHER), 2), (source, Target::Player(YOU), 1)];
    assert_eq!(damage_together(&mut cx, &hits), vec![0, 2, 1]);
    assert_eq!(cx.s().player(LEFT).life, 20);
    assert_eq!(cx.s().player(OTHER).life, -2, "zero life is not departure before state-based actions");
    assert_eq!(cx.s().player(YOU).life, 22);
    let events: Vec<Target> = cx.g.log.iter().filter_map(|r| match r.event { Event::Damage { target, .. } => Some(target), _ => None }).collect();
    assert_eq!(events, vec![Target::Player(OTHER), Target::Player(YOU)]);
}

#[test]
fn a_departed_lifelink_controller_gains_nothing_from_live_damage() {
    let (mut game, source) = game();
    game.state.update_obj(source, |o| o.controller = LEFT);
    let mut cx = cx(&mut game);
    assert_eq!(damage(&mut cx, source, Target::Player(OTHER), 2, false), 2);
    assert_eq!(cx.s().player(OTHER).life, 18);
    assert_eq!(cx.s().player(LEFT).life, 20);
    assert!(!cx.g.log.iter().any(|r| matches!(r.event, Event::LifeChange { player: LEFT, .. })));
}

#[test]
fn invalid_redirection_keeps_original_live_damage_and_does_not_use_destination_shield() {
    let (mut game, source) = game();
    let id = game.state.push_obj(Obj::new(game.cards.id_of(&REDIRECT), YOU, Zone::BATTLEFIELD));
    game.state.zone_push(Zone::BATTLEFIELD, id);
    let card = game.cards.id_of(&SHIELD);
    game.state.effect_push(ContinuousEffect {
        source, affected: source, controller: YOU,
        kind: EffectKind::Prevention { card, index: 0, to: Target::Player(LEFT), shield: Shield::Next(3) },
        duration: Duration::EndOfTurn, timestamp: 1,
    });
    let mut cx = cx(&mut game);
    assert_eq!(damage(&mut cx, source, Target::Player(OTHER), 2, false), 2);
    assert_eq!(cx.s().player(OTHER).life, 18);
    assert_eq!(cx.s().player(YOU).life, 22);
    assert_eq!(cx.g.log.iter().filter(|r| matches!(r.event, Event::Damage { target: Target::Player(OTHER), amount: 2, .. })).count(), 1);
    assert!(!cx.g.log.iter().any(|r| matches!(r.event, Event::DamagePrevented { .. })));
    assert!(matches!(cx.s().effects[0].kind, EffectKind::Prevention { shield: Shield::Next(3), .. }));
}
