//! Toxic (CR 702.164): combat damage dealt to a player by a creature with toxic also gives that player poison counters equal
//! to the creature's total toxic value, the sum of every toxic N it has (CR 702.164b-c, 120.3g), whatever the amount dealt;
//! damage that is not dealt gives none, nor does noncombat damage or damage to a permanent. The counters are put by the
//! creature's controller, through the counter replacement effects. Over test cards: creatures with toxic 2, with infect and
//! toxic 1, a planeswalker, and enchantments that double the poison counters put on a player or prevent damage.

use crate::{Cx, Game, Setup, layers, ops};
use cantrip_api::*;
use cantrip_state::{Frame, FrameKind};

const YOU: PlayerId = PlayerId(0);
const THEM: PlayerId = PlayerId(1);

static TOXIC: CardDef = CardDef {
    name: "Toxic Test Two",
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(3),
    toughness: Some(3),
    toxic: 2,
    ..CardDef::BLANK
};

static INFECT_TOXIC: CardDef =
    CardDef { name: "Toxic Test Infect", keywords: Keywords::of(&[Keyword::Infect]), toxic: 1, ..TOXIC };

static WALKER: CardDef = CardDef {
    name: "Toxic Test Walker",
    types: TypeSet::of(&[CardType::Planeswalker]),
    loyalty: Some(5),
    ..CardDef::BLANK
};

/// "If one or more poison counters would be put on a player, twice that many are put on them instead."
fn poison_on_a_player(_: &dyn Query, _: &Asker, proposal: &Proposal) -> bool {
    matches!(proposal, Proposal::PlayerCounters(c) if c.poison > 0)
}

fn twice_the_poison(_: &dyn Query, _: &Asker, proposal: &mut Proposal) {
    if let Proposal::PlayerCounters(c) = proposal {
        c.poison *= 2;
    }
}

static DOUBLER: CardDef = CardDef {
    name: "Toxic Test Doubler",
    types: TypeSet::of(&[CardType::Enchantment]),
    replacements: &[replacement(ReplacementKind::Counters, poison_on_a_player, twice_the_poison, "twice the poison counters")],
    ..CardDef::BLANK
};

/// "If damage would be dealt to a player, prevent all but 1 of it" (a test's shield): `KEEP` of it is dealt.
fn to_a_player(_: &dyn Query, _: &Asker, proposal: &Proposal) -> bool {
    matches!(proposal, Proposal::Damage(d) if matches!(d.target, Target::Player(_)))
}

fn keep<const KEEP: u32>(_: &dyn Query, _: &Asker, proposal: &mut Proposal) {
    if let Proposal::Damage(d) = proposal {
        d.amount = d.amount.min(u32::from(KEEP));
    }
}

static PREVENT_ALL: CardDef = CardDef {
    name: "Toxic Test Prevent All",
    types: TypeSet::of(&[CardType::Enchantment]),
    replacements: &[replacement(ReplacementKind::Damage, to_a_player, keep::<0>, "prevent all damage to players")],
    ..CardDef::BLANK
};

static PREVENT_ALL_BUT_ONE: CardDef = CardDef {
    name: "Toxic Test Prevent All But One",
    types: TypeSet::of(&[CardType::Enchantment]),
    replacements: &[replacement(ReplacementKind::Damage, to_a_player, keep::<1>, "prevent all but 1 damage to players")],
    ..CardDef::BLANK
};

fn game() -> Game {
    let table: &'static CardTable =
        Box::leak(Box::new(CardTable::new([&TOXIC, &INFECT_TOXIC, &WALKER, &DOUBLER, &PREVENT_ALL, &PREVENT_ALL_BUT_ONE])));
    let mut g = Game::new(table, &Setup { decks: vec![Vec::new(), Vec::new()], ..Default::default() });
    while !g.state.agenda.is_empty() {
        g.state.frame_remove(0);
    }
    g.state.frame_push(Frame::new(FrameKind::Priority));
    g
}

fn put(g: &mut Game, def: &'static CardDef, controller: PlayerId) -> ObjectId {
    let id = g.state.push_obj(Obj::new(g.cards.id_of(def), controller, Zone::BATTLEFIELD));
    let timestamp = g.state.timestamp();
    g.state.update_obj(id, |o| o.timestamp = timestamp);
    g.state.zone_push(Zone::BATTLEFIELD, id);
    g.state.see_statics();
    id
}

fn cx(g: &mut Game) -> Cx<'_> {
    Cx { g, frame: 0, cursor: 0, mana_source: None, entering: None, lineage: Vec::new(), damage: None, asked_at: None, doer: None, producing: None }
}

/// `source` deals `amount` damage to `target`, combat damage or not, as one event of its own.
fn deal(g: &mut Game, source: ObjectId, target: Target, amount: u32, combat: bool) {
    let mut cx = cx(g);
    let mut batch = ops::DamageBatch::default();
    let mut events = ops::deal_damage_with(&mut cx, source, target, amount, combat, &mut batch).into_iter().collect();
    batch.finish(&mut cx, &mut events);
    batch.rest_of_preventions(&mut cx);
}

fn grant(g: &mut Game, id: ObjectId, kind: EffectKind) {
    let mut cx = cx(g);
    cantrip_api::Game::add_effect(&mut cx, id, id, kind, Duration::EndOfTurn);
}

fn total_toxic(g: &Game, id: ObjectId) -> u16 {
    layers::chars(g.cards, &g.state, &g.log, id).toxic
}

#[test]
fn combat_damage_to_a_player_gives_the_total_toxic_value_whatever_the_amount() {
    let mut g = game();
    let creature = put(&mut g, &TOXIC, YOU);
    let life = g.state.player(THEM).life;
    deal(&mut g, creature, Target::Player(THEM), 3, true);
    assert_eq!(g.state.player(THEM).poison(), 2);
    assert_eq!(g.state.player(THEM).life, life - 3, "the damage's other results still happen");
    deal(&mut g, creature, Target::Player(THEM), 1, true);
    assert_eq!(g.state.player(THEM).poison(), 4);
}

#[test]
fn every_instance_of_toxic_adds_to_the_total() {
    let mut g = game();
    let creature = put(&mut g, &TOXIC, YOU);
    assert_eq!(total_toxic(&g, creature), 2);
    grant(&mut g, creature, EffectKind::Toxic(1));
    grant(&mut g, creature, EffectKind::Toxic(1));
    assert_eq!(total_toxic(&g, creature), 4);
    deal(&mut g, creature, Target::Player(THEM), 3, true);
    assert_eq!(g.state.player(THEM).poison(), 4);
}

#[test]
fn losing_all_abilities_loses_toxic_and_what_is_granted_after_is_kept() {
    let mut g = game();
    let creature = put(&mut g, &TOXIC, YOU);
    grant(&mut g, creature, EffectKind::LoseAllAbilities);
    assert_eq!(total_toxic(&g, creature), 0);
    deal(&mut g, creature, Target::Player(THEM), 3, true);
    assert_eq!(g.state.player(THEM).poison(), 0);
    grant(&mut g, creature, EffectKind::Toxic(1));
    assert_eq!(total_toxic(&g, creature), 1);
}

#[test]
fn noncombat_damage_gives_no_poison() {
    let mut g = game();
    let creature = put(&mut g, &TOXIC, YOU);
    deal(&mut g, creature, Target::Player(THEM), 3, false);
    assert_eq!(g.state.player(THEM).poison(), 0);
}

#[test]
fn prevented_damage_gives_no_poison_and_partly_prevented_gives_it_all() {
    let mut g = game();
    let creature = put(&mut g, &TOXIC, YOU);
    put(&mut g, &PREVENT_ALL, THEM);
    deal(&mut g, creature, Target::Player(THEM), 3, true);
    assert_eq!(g.state.player(THEM).poison(), 0);

    let mut g = game();
    let creature = put(&mut g, &TOXIC, YOU);
    put(&mut g, &PREVENT_ALL_BUT_ONE, THEM);
    let life = g.state.player(THEM).life;
    deal(&mut g, creature, Target::Player(THEM), 3, true);
    assert_eq!(g.state.player(THEM).poison(), 2);
    assert_eq!(g.state.player(THEM).life, life - 1);
}

#[test]
fn combat_damage_to_a_planeswalker_gives_no_poison() {
    let mut g = game();
    let creature = put(&mut g, &TOXIC, YOU);
    let walker = put(&mut g, &WALKER, THEM);
    deal(&mut g, creature, Target::Object(walker), 3, true);
    assert_eq!(g.state.player(THEM).poison(), 0);
    assert_eq!(g.state.player(YOU).poison(), 0);
}

#[test]
fn the_poison_counters_go_through_the_counter_replacement_effects() {
    let mut g = game();
    let creature = put(&mut g, &TOXIC, YOU);
    put(&mut g, &DOUBLER, THEM);
    deal(&mut g, creature, Target::Player(THEM), 1, true);
    assert_eq!(g.state.player(THEM).poison(), 4);
}

#[test]
fn infect_and_toxic_give_both_their_counters_as_one_put() {
    let mut g = game();
    let creature = put(&mut g, &INFECT_TOXIC, YOU);
    let life = g.state.player(THEM).life;
    deal(&mut g, creature, Target::Player(THEM), 3, true);
    assert_eq!(g.state.player(THEM).poison(), 4);
    assert_eq!(g.state.player(THEM).life, life);
}
