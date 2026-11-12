//! Static group rules keep source context and full declaration membership.
use crate::{Cx, Game, Setup, static_rules};
use cantrip_api::{Game as Rules, *};
const A: PlayerId = PlayerId(0);
const B: PlayerId = PlayerId(1);
static CAP: CardDef = CardDef {
    name: "Combat cap fixture",
    combat_groups: &[CombatGroupRule { role: CombatRole::Attacker, at_most: 2 }, CombatGroupRule { role: CombatRole::Blocker, at_most: 2 }],
    ..CardDef::BLANK
};
static BEAR: CardDef = CardDef { name: "Combat group bear", types: TypeSet::of(&[CardType::Creature]), ..CardDef::BLANK };
fn game() -> Game {
    let t = Box::leak(Box::new(CardTable::new([&CAP, &BEAR])));
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
        reached: None,
    }
}
#[test]
fn global_groups_share_membership_and_disappear_with_the_static_source() {
    let mut g = game();
    let cap = put(&mut g, &CAP, B);
    let other = put(&mut g, &CAP, A);
    let ids: Vec<_> = (0..4).map(|_| put(&mut g, &BEAR, A)).collect();
    for role in [CombatRole::Attacker, CombatRole::Blocker] {
        let v = static_rules::combat_groups(&cx(&mut g), role, &ids);
        assert_eq!(v, [CombatGroup { members: ids.clone(), at_most: 2 }], "two of the same cap are one");
    }
    cx(&mut g).add_effect(cap, cap, EffectKind::LoseAllAbilities, Duration::EndOfTurn);
    assert_eq!(static_rules::combat_groups(&cx(&mut g), CombatRole::Attacker, &ids).len(), 1);
    cx(&mut g).add_effect(other, other, EffectKind::LoseAllAbilities, Duration::EndOfTurn);
    assert!(static_rules::combat_groups(&cx(&mut g), CombatRole::Attacker, &ids).is_empty());
}
#[test]
fn unused_definitions_do_not_create_declaration_constraints() {
    let mut g = game();
    let b = put(&mut g, &BEAR, A);
    assert!(static_rules::combat_groups(&cx(&mut g), CombatRole::Attacker, &[b]).is_empty());
}
