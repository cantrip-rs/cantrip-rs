//! A resolved rule that damage dealt to a permanent of a set can't be prevented (`LastingRuleKind::DamageCantBePrevented`, CR
//! 615.12, 611.2c) or dealt instead to another permanent or player (`LastingRuleKind::DamageCantBeRedirected`, CR 614.9), over
//! test cards: damage dealt to two creatures at once, one of them in the set.
use crate::{Cx, Game, Setup, StateCheck};
use cantrip_api::Game as Rules;
use cantrip_api::*;
use cantrip_state::{Frame, FrameKind};

const YOU: PlayerId = PlayerId(0);
const THEM: PlayerId = PlayerId(1);

static SOURCE: CardDef = CardDef { name: "Resolved damage rule source", types: TypeSet::of(&[CardType::Instant]), ..CardDef::BLANK };
static BEAR: CardDef = CardDef {
    name: "Resolved damage rule creature",
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(5),
    toughness: Some(5),
    ..CardDef::BLANK
};

/// The one creature the effect's words named.
fn that_creature(_: &dyn Query, _: &Asker, about: Target, id: ObjectId) -> bool {
    about == Target::Object(id)
}
static RULES: CardDef = CardDef {
    name: "Resolved damage rule",
    types: TypeSet::of(&[CardType::Sorcery]),
    lasting_rules: &[
        LastingRule {
            text: "Damage that would be dealt to that creature this turn can't be prevented.",
            rule: LastingRuleKind::DamageCantBePrevented,
            applies: that_creature,
        },
        LastingRule {
            text: "Damage that would be dealt to that creature this turn can't be dealt instead to another permanent or player.",
            rule: LastingRuleKind::DamageCantBeRedirected,
            applies: that_creature,
        },
    ],
    ..CardDef::BLANK
};

fn to_a_creature(q: &dyn Query, _: &Asker, d: &DamageEvent) -> bool {
    matches!(d.target, Target::Object(id) if q.chars(id).is(CardType::Creature))
}
/// "Prevent all damage that would be dealt to creatures."
static PREVENT: CardDef = CardDef {
    name: "Resolved damage rule prevention",
    types: TypeSet::of(&[CardType::Enchantment]),
    preventions: &[PreventionDef { hint: "prevent all", applies: to_a_creature, amount: PreventionAmount::All, then: None }],
    ..CardDef::BLANK
};
fn damage_to_a_creature(q: &dyn Query, a: &Asker, p: &Proposal) -> bool {
    matches!(p, Proposal::Damage(d) if to_a_creature(q, a, d) && d.amount > 0)
}
fn to_you(_: &dyn Query, a: &Asker, p: &mut Proposal) {
    if let Proposal::Damage(d) = p {
        d.target = Target::Player(a.controller);
    }
}
/// "Damage that would be dealt to a creature is dealt to you instead."
static REDIRECT: CardDef = CardDef {
    name: "Resolved damage rule redirection",
    types: TypeSet::of(&[CardType::Enchantment]),
    replacements: &[replacement(ReplacementKind::Damage, damage_to_a_creature, to_you, "dealt to you instead")],
    ..CardDef::BLANK
};

fn game() -> Game {
    let cards = Box::leak(Box::new(CardTable::new([&SOURCE, &BEAR, &RULES, &PREVENT, &REDIRECT])));
    let mut g = Game::new(cards, &Setup { decks: vec![vec![cards.id_of(&SOURCE); 20]; 2], ..Default::default() });
    while !g.state.agenda.is_empty() {
        g.state.frame_remove(0);
    }
    g.state.frame_push(Frame::new(FrameKind::BeginStep));
    g.state.update_turn(|t| {
        t.number = 1;
        t.active = YOU;
    });
    g.verify = true;
    g
}
fn put(g: &mut Game, card: &'static CardDef, controller: PlayerId) -> ObjectId {
    let id = g.state.push_obj(Obj::new(g.cards.id_of(card), controller, Zone::BATTLEFIELD));
    g.state.zone_push(Zone::BATTLEFIELD, id);
    g.state.see_statics();
    id
}
fn cx(g: &mut Game, source: ObjectId) -> Cx<'_> {
    g.check = StateCheck { depth: 1, resolving: Some(source), ..StateCheck::NONE };
    Cx {
        g,
        frame: 0,
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

/// THEM's two creatures are each dealt 2 damage at once, with `modifier` on the battlefield under THEM and the rules made about
/// the first creature: the damage marked on each, and THEM's life.
fn hit(modifier: &'static CardDef) -> (u32, u32, i32) {
    let mut g = game();
    let source = put(&mut g, &SOURCE, YOU);
    let named = put(&mut g, &BEAR, THEM);
    let other = put(&mut g, &BEAR, THEM);
    put(&mut g, modifier, THEM);
    let mut cx = cx(&mut g, source);
    let rules = cx.cards().id_of(&RULES);
    instruction(&mut cx, |g| {
        for index in 0..2 {
            g.add_player_effect(YOU, source, EffectKind::LastingRule { card: rules, index, about: Target::Object(named) }, Duration::EndOfTurn);
        }
    });
    cx.end_sentence();
    cx.damage_together(&[(source, Target::Object(named), 2), (source, Target::Object(other), 2)]);
    (cx.obj(named).damage, cx.obj(other).damage, cx.state().player(THEM).life)
}

/// CR 615.12, 614.9, 611.2c: the damage dealt to the creature the rules are about is neither prevented nor dealt to THEM
/// instead; the damage dealt to the other creature at the same time is.
#[test]
fn a_resolved_rule_keeps_damage_to_its_creature_from_prevention_and_redirection() {
    assert_eq!(hit(&PREVENT), (2, 0, 20), "only the other creature's damage is prevented");
    assert_eq!(hit(&REDIRECT), (2, 0, 18), "only the other creature's damage is dealt to THEM instead");
}
