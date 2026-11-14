//! Sparse token-creation provenance for linked abilities (CR 607.2c, 607.5).
use crate::{Cx, Game, Setup};
use cantrip_api::*;
use cantrip_state::{Frame, FrameKind};

const P: PlayerId = PlayerId(0);
static SOURCE: CardDef = CardDef { name: "Linked creation source", types: TypeSet::of(&[CardType::Artifact]), ..CardDef::BLANK };
static TOKEN: CardDef = CardDef { name: "Linked creation token", types: TypeSet::of(&[CardType::Creature]), power: Some(1), toughness: Some(1), ..CardDef::BLANK };
fn gain(_: &dyn Query, _: &Asker, p: &Proposal) -> bool { matches!(p, Proposal::LifeChange(l) if l.amount > 0) }
fn gain_more(_: &dyn Query, _: &Asker, p: &mut Proposal) { if let Proposal::LifeChange(l) = p { l.amount += 1; } }
fn replacement_token(g: &mut dyn cantrip_api::Game, a: &Asker, _: &Proposal, _: &Proposal) {
    let token = g.card_id(&TOKEN);
    g.create_tokens(token, a.controller, 1, false).unwrap();
}
static REPLACE: CardDef = CardDef { name: "Unrelated creation replacement", types: TypeSet::of(&[CardType::Enchantment]), replacements: &[replacement(ReplacementKind::LifeChange, gain, gain_more, "gain more and make a token").then(replacement_token)], ..CardDef::BLANK };
fn cx(g: &mut Game) -> Cx<'_> { Cx { g, frame: 0, cursor: 0, mana_source: None, entering: None, lineage: vec![], damage: None, asked_at: None, doer: None, producing: None } }
#[test]
fn created_tokens_capture_exact_link_and_specific_creating_ability() {
    let cards = Box::leak(Box::new(CardTable::new([&SOURCE, &TOKEN, &REPLACE])));
    let mut g = Game::new(cards, &Setup { decks: vec![vec![], vec![]], ..Default::default() });
    while !g.state.agenda.is_empty() { g.state.frame_remove(0); }
    let source = g.state.push_obj(Obj::new(cards.id_of(&SOURCE), P, Zone::BATTLEFIELD));
    g.state.zone_push(Zone::BATTLEFIELD, source);
    let ability = AbilityRef { card: cards.id_of(&SOURCE), index: 0 };
    let mut item = crate::actions::stack_item(StackKind::Triggered, source, ability, P, &[]);
    item.acquired = Acquired { by: AcquiredBy::Grant, stamp: 7, part: 1 };
    g.state.stack_push(item);
    let serial = g.state.stack.last().unwrap().serial;
    g.state.frame_push(Frame::new(FrameKind::Resolve { serial }));
    let token_id = cards.id_of(&TOKEN);
    let mark = g.state.mark();
    let history_mark = g.log.len();
    let created = crate::ops::create_tokens(&mut cx(&mut g), token_id, P, 2, false).unwrap();
    let by = Linked { object: source, text: cards.linked_card(ability.card), acquired: item.acquired };
    let got: Vec<_> = History::new(&g.log, 1).tokens_created_by(by, ability).collect();
    assert_eq!(got, created);
    for &token in &created {
        assert!(g.log.iter().any(|r| matches!(r.event, Event::ZoneChange { old, new, reason: ZoneReason::Created, .. } if old == token && new == token)));
    }
    g.state.revert(mark);
    g.log.truncate(history_mark);
    assert_eq!(History::new(&g.log, 1).tokens_created_by(by, ability).count(), 0);
    let remade = crate::ops::create_tokens(&mut cx(&mut g), token_id, P, 2, false).unwrap();
    assert_eq!(History::new(&g.log, 1).tokens_created_by(by, ability).collect::<Vec<_>>(), remade);
    g.check.replacement_callback = true;
    let unrelated = crate::ops::create_tokens(&mut cx(&mut g), token_id, P, 1, false).unwrap();
    g.check.replacement_callback = false;
    let replacement = g.state.push_obj(Obj::new(cards.id_of(&REPLACE), P, Zone::BATTLEFIELD));
    g.state.zone_push(Zone::BATTLEFIELD, replacement);
    cantrip_api::Game::gain_life(&mut cx(&mut g), P, 1);
    let after_callback: Vec<_> = History::new(&g.log, 1).tokens_created_by(by, ability).collect();
    assert_eq!(after_callback, remade, "a replacement's own token is not the resolving ability's creation");
    let mut gained_again = *g.state.stack.last().unwrap();
    gained_again.acquired.stamp += 1;
    g.state.stack_set(0, gained_again);
    let separately_made = crate::ops::create_tokens(&mut cx(&mut g), token_id, P, 1, false).unwrap();
    assert!(!History::new(&g.log, 1).tokens_created_by(by, ability).any(|id| separately_made.contains(&id)));
    let later_by = Linked { acquired: gained_again.acquired, ..by };
    assert_eq!(History::new(&g.log, 1).tokens_created_by(later_by, ability).collect::<Vec<_>>(), separately_made);
    assert!(!History::new(&g.log, 1).tokens_created_by(by, ability).any(|id| unrelated.contains(&id)));
    let mut other = by; other.acquired.stamp += 2;
    assert_eq!(History::new(&g.log, 1).tokens_created_by(other, ability).count(), 0);
    assert_eq!(History::new(&g.log, 1).tokens_created_by(by, AbilityRef { index: 1, ..ability }).count(), 0);
    assert_eq!(std::mem::size_of::<crate::StateCheck>(), 16);
    assert_eq!(std::mem::size_of::<Event>(), 28);
    assert_eq!(std::mem::size_of::<cantrip_api::history::Record>(), 52);
}
