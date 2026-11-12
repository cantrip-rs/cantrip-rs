//! Static group rules keep source context and full declaration membership.
use crate::{Cx, Game, Setup, static_rules};
use cantrip_api::{Game as Rules, *};
const A: PlayerId = PlayerId(0);
const B: PlayerId = PlayerId(1);
fn any(_: &dyn Query, _: &Asker, _: ObjectId) -> bool {
    true
}
fn own(_: &dyn Query, a: &Asker, id: ObjectId) -> bool {
    id == a.source
}
fn everyone(_: &dyn Query, _: &Asker, _: ObjectId, _: ObjectId) -> bool {
    true
}
fn yours(q: &dyn Query, a: &Asker, _: ObjectId, other: ObjectId) -> bool {
    q.controller(other) == a.controller
}
static CAP: CardDef = CardDef {
    name: "Combat cap fixture",
    combat_groups: &[
        CombatGroupRule { role: CombatRole::Attacker, applies: any, counted: everyone, others_only: false, at_least: 0, at_most: 2 },
        CombatGroupRule { role: CombatRole::Blocker, applies: any, counted: everyone, others_only: false, at_least: 0, at_most: 2 },
    ],
    ..CardDef::BLANK
};
static CONSCRIPTS: CardDef = CardDef {
    name: "Combat companion fixture",
    combat_groups: &[CombatGroupRule { role: CombatRole::Attacker, applies: own, counted: yours, others_only: true, at_least: 2, at_most: u32::MAX }],
    types: TypeSet::of(&[CardType::Creature]),
    ..CardDef::BLANK
};
static BEAR: CardDef = CardDef { name: "Combat group bear", types: TypeSet::of(&[CardType::Creature]), ..CardDef::BLANK };
fn game() -> Game {
    let t = Box::leak(Box::new(CardTable::new([&CAP, &CONSCRIPTS, &BEAR])));
    Game::new(t, &Setup { decks: vec![vec![], vec![]], ..Default::default() })
}
fn put(g: &mut Game, d: &'static CardDef, p: PlayerId) -> ObjectId {
    let id = g.state.push_obj(Obj::new(g.cards.id_of(d), p, Zone::BATTLEFIELD));
    g.state.zone_push(Zone::BATTLEFIELD, id);
    id
}
fn cx(g: &mut Game) -> Cx<'_> {
    Cx {
        g,
        frame: usize::MAX,
        cursor: 0,
        mana_source: None,
        entering: None,
        lineage: vec![],
        damage: None,
        asked_at: None,
        doer: None,
        producing: None,
    }
}
#[test]
fn global_groups_share_membership_and_disappear_with_the_static_source() {
    let mut g = game();
    let cap = put(&mut g, &CAP, B);
    let ids: Vec<_> = (0..4).map(|_| put(&mut g, &BEAR, A)).collect();
    for role in [CombatRole::Attacker, CombatRole::Blocker] {
        let v = static_rules::combat_groups(&cx(&mut g), role, &ids);
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].members, ids);
        assert_eq!(v[0].subjects, ids);
    }
    cx(&mut g).add_effect(cap, cap, EffectKind::LoseAllAbilities, Duration::EndOfTurn);
    assert!(static_rules::combat_groups(&cx(&mut g), CombatRole::Attacker, &ids).is_empty());
}
#[test]
fn companion_membership_uses_controller_and_excludes_the_subject() {
    let mut g = game();
    let source = put(&mut g, &CONSCRIPTS, A);
    let own = put(&mut g, &BEAR, A);
    let enemy = put(&mut g, &BEAR, B);
    let ids = [source, own, enemy];
    let v = static_rules::combat_groups(&cx(&mut g), CombatRole::Attacker, &ids);
    assert_eq!(v.len(), 1);
    assert_eq!(v[0].members, [own]);
    g.state.update_obj(source, |o| o.controller = B);
    let v = static_rules::combat_groups(&cx(&mut g), CombatRole::Attacker, &ids);
    assert_eq!(v[0].members, [enemy]);
    assert!(static_rules::combat_groups(&cx(&mut g), CombatRole::Blocker, &ids).is_empty());
}
#[test]
fn unused_definitions_do_not_create_declaration_constraints() {
    let mut g = game();
    let b = put(&mut g, &BEAR, A);
    assert!(static_rules::combat_groups(&cx(&mut g), CombatRole::Attacker, &[b]).is_empty());
}
