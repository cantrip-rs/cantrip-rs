//! CR702.22c versus j/k: one qualifying provider, any number of matching peers;
//! damage assignment needs a matching pair, not a homogeneous whole group.
use crate::{Cx, Game, Setup, banding};
use cantrip_api::Game as Rules;
use cantrip_api::*;
use cantrip_state::{Frame, FrameKind};
const A: PlayerId = PlayerId(0);
const B: PlayerId = PlayerId(1);
const C: PlayerId = PlayerId(2);
fn legendary(q: &dyn Query, id: ObjectId) -> bool {
    q.chars(id).is(CardType::Creature) && q.chars(id).supertypes.legendary
}
fn wolf(q: &dyn Query, id: ObjectId) -> bool {
    q.chars(id).is(CardType::Creature) && q.chars(id).name == "Wolves of the Hunt"
}
static PLAIN: CardDef = CardDef {
    name: "Bands plain creature",
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(2),
    toughness: Some(2),
    ..CardDef::BLANK
};
static LEGEND: CardDef = CardDef {
    name: "Bands ordinary legendary",
    supertypes: Supertypes {
        legendary: true,
        ..CardDef::BLANK.supertypes
    },
    ..PLAIN
};
static PROVIDER: CardDef = CardDef {
    name: "Bands legendary provider",
    abilities: &[cantrip_mechanics::banding::with_others(
        legendary,
        "Bands with other legendary creatures.",
    )],
    ..LEGEND
};
static WRONG: CardDef = CardDef {
    name: "Bands nonqualifying provider",
    abilities: PROVIDER.abilities,
    ..PLAIN
};
static WOLF: CardDef = CardDef {
    name: "Wolves of the Hunt",
    abilities: &[cantrip_mechanics::banding::with_others(
        wolf,
        "Bands with other creatures named Wolves of the Hunt.",
    )],
    ..PLAIN
};
static ORDINARY: CardDef = CardDef {
    name: "Bands ordinary bander",
    keywords: Keywords::of(&[Keyword::Banding]),
    ..LEGEND
};
static GIVER: CardDef = CardDef {
    name: "Bands granting fixture",
    delayed: PROVIDER.abilities,
    ..PLAIN
};
static HUGE: CardDef = CardDef {
    name: "Bands assignment source",
    power: Some(9),
    toughness: Some(9),
    keywords: Keywords::of(&[Keyword::Trample]),
    abilities: &[block_allowance(2, "Can block two additional creatures.")],
    ..PLAIN
};
static SPELL: CardDef = CardDef {
    name: "Bands priority holder",
    types: TypeSet::of(&[CardType::Instant]),
    has_cost: true,
    spell: spell(&[], |_, _| Ok(())),
    ..CardDef::BLANK
};
fn board() -> Game {
    let cards = Box::leak(Box::new(CardTable::new([
        &PLAIN, &LEGEND, &PROVIDER, &WRONG, &WOLF, &ORDINARY, &GIVER, &HUGE, &SPELL,
    ])));
    let fill = cards.id_of(&PLAIN);
    let mut g = Game::new(
        cards,
        &Setup {
            decks: vec![vec![fill; 20]; 3],
            shuffle: false,
            ..Default::default()
        },
    );
    while !g.state.agenda.is_empty() {
        g.state.frame_remove(0);
    }
    g.state.frame_push(Frame::new(FrameKind::BeginStep));
    g.state.update_turn(|t| {
        t.number = 2;
        t.active = A;
        t.priority = A;
        t.set_step(Step::DeclareAttackers);
    });
    g.state.update_player(A, |p| p.turn_began = 2);
    let id = g
        .state
        .push_obj(Obj::new(cards.id_of(&SPELL), A, Zone::hand(A)));
    g.state.zone_push(Zone::hand(A), id);
    g
}
fn put(g: &mut Game, card: &'static CardDef, p: PlayerId) -> ObjectId {
    let id = g
        .state
        .push_obj(Obj::new(g.cards.id_of(card), p, Zone::BATTLEFIELD));
    g.state.zone_push(Zone::BATTLEFIELD, id);
    g.state.see_statics();
    id
}
fn cx(g: &mut Game) -> Cx<'_> {
    Cx {
        g,
        frame: 0,
        cursor: 0,
        mana_source: None,
        entering: None,
        lineage: Vec::new(),
        damage: None,
        asked_at: None,
        doer: None,
        producing: None,
    }
}
fn choice(g: &Game, source: ObjectId, candidates: &[ObjectId], selected: &[ObjectId]) -> Verdict {
    banding::selection(
        g,
        &Choice {
            chooser: A,
            source,
            candidates,
            selected,
        },
        0,
    )
}
fn force_step(g: &mut Game, step: Step) {
    while !g.state.agenda.is_empty() {
        g.state.frame_remove(0);
    }
    g.state.frame_push(Frame::new(FrameKind::BeginStep));
    g.state.update_turn(|t| {
        t.set_step(step);
        t.priority = A;
    });
}
#[test]
fn one_other_band_provider_accepts_many_matching_peers_without_giving_them_the_ability() {
    let mut g = board();
    let p = put(&mut g, &PROVIDER, A);
    let x = put(&mut g, &LEGEND, A);
    let y = put(&mut g, &LEGEND, A);
    let out = put(&mut g, &PLAIN, A);
    assert!(!g.chars(p).has(Keyword::Banding));
    assert!(!banding::can_lead(&g, x));
    assert!(banding::can_lead(&g, p));
    let verdict = choice(&g, p, &[x, y, out], &[x, y]);
    assert!(verdict.complete);
    assert!(!verdict.addable.get(2));
    assert!(!choice(&g, p, &[x, y, out], &[x, out]).complete);
}
#[test]
fn the_named_quality_is_real_and_the_provider_must_itself_match_it() {
    let mut g = board();
    let wrong = put(&mut g, &WRONG, A);
    let legend = put(&mut g, &LEGEND, A);
    let wolf1 = put(&mut g, &WOLF, A);
    let wolf2 = put(&mut g, &WOLF, A);
    assert!(!banding::can_lead(&g, wrong));
    assert!(!choice(&g, wrong, &[legend], &[legend]).complete);
    assert!(choice(&g, wolf1, &[wolf2, legend], &[wolf2]).complete);
    assert!(!choice(&g, wolf1, &[wolf2, legend], &[legend]).complete);
}
#[test]
fn partial_selection_can_be_completed_by_an_unused_other_band_provider() {
    let mut g = board();
    let ordinary = put(&mut g, &ORDINARY, A);
    let x = put(&mut g, &LEGEND, A);
    let y = put(&mut g, &LEGEND, A);
    let provider = put(&mut g, &PROVIDER, A);
    let v = choice(&g, ordinary, &[x, y, provider], &[x, y]);
    assert!(!v.complete);
    assert!(v.addable.get(2));
    assert!(choice(&g, ordinary, &[x, y, provider], &[x, y, provider]).complete);
    assert!(choice(&g, ordinary, &[x, y], &[x]).complete);
    assert!(!choice(&g, ordinary, &[x, y], &[x, y]).complete);
}
#[test]
fn assignment_control_needs_a_pair_but_allows_other_nonmatching_combatants() {
    let mut g = board();
    let p = put(&mut g, &PROVIDER, A);
    let peer = put(&mut g, &LEGEND, A);
    let out = put(&mut g, &PLAIN, A);
    let wrong = put(&mut g, &WRONG, A);
    assert!(!banding::controls_assignment(&g, &[p]));
    assert!(!banding::controls_assignment(&g, &[p, out]));
    assert!(banding::controls_assignment(&g, &[p, peer, out]));
    assert!(!banding::controls_assignment(&g, &[wrong, peer]));
}
#[test]
fn family_loss_removes_all_kinds_but_keeps_ordinary_banding_and_later_grants() {
    let mut g = board();
    let p = put(&mut g, &ORDINARY, A);
    let giver = put(&mut g, &GIVER, A);
    let given = AbilityRef::delayed(g.cards.id_of(&GIVER), 0);
    cx(&mut g).add_effect(
        giver,
        p,
        EffectKind::AddAbility(given),
        Duration::Indefinite,
    );
    assert!(g.chars(p).abilities().any(|a| a == given));
    cx(&mut g).add_effect(
        giver,
        p,
        EffectKind::LoseBandsWithOthers,
        Duration::EndOfTurn,
    );
    assert!(g.chars(p).has(Keyword::Banding));
    assert!(!g.chars(p).abilities().any(|a| a == given));
    cx(&mut g).add_effect(
        giver,
        p,
        EffectKind::AddAbility(given),
        Duration::Indefinite,
    );
    assert!(g.chars(p).abilities().any(|a| a == given));
}
#[test]
fn losing_ordinary_banding_also_loses_every_other_kind_even_without_an_ordinary_instance() {
    let mut g = board();
    let p = put(&mut g, &PROVIDER, A);
    let giver = put(&mut g, &GIVER, A);
    let named = AbilityRef {
        card: g.cards.id_of(&WOLF),
        index: 0,
    };
    cx(&mut g).add_effect(
        giver,
        p,
        EffectKind::AddAbility(named),
        Duration::Indefinite,
    );
    assert_eq!(g.chars(p).abilities().count(), 2);
    assert!(!g.chars(p).has(Keyword::Banding));
    cx(&mut g).add_effect(
        giver,
        p,
        EffectKind::LoseKeywords(Keywords::of(&[Keyword::Flying])),
        Duration::EndOfTurn,
    );
    assert_eq!(g.chars(p).abilities().count(), 2);
    cx(&mut g).add_effect(
        giver,
        p,
        EffectKind::LoseKeywords(Keywords::of(&[Keyword::Banding])),
        Duration::EndOfTurn,
    );
    assert_eq!(g.chars(p).abilities().count(), 0);
    assert!(!banding::can_lead(&g, p));
    cx(&mut g).add_effect(
        giver,
        p,
        EffectKind::AddKeywords(Keywords::of(&[Keyword::Banding])),
        Duration::EndOfTurn,
    );
    assert!(g.chars(p).has(Keyword::Banding));
    assert_eq!(g.chars(p).abilities().count(), 0);
    let given = AbilityRef::delayed(g.cards.id_of(&GIVER), 0);
    cx(&mut g).add_effect(giver, p, EffectKind::AddAbility(given), Duration::EndOfTurn);
    assert_eq!(g.chars(p).abilities().count(), 1);
}
#[test]
fn copying_and_losing_abilities_change_the_keyword_without_changing_ordinary_banding() {
    let mut g = board();
    let p = put(&mut g, &LEGEND, A);
    let card = g.cards.id_of(&PROVIDER);
    cx(&mut g).set_copy(p, Copiable::of(card));
    assert!(banding::can_lead(&g, p));
    cx(&mut g).add_effect(p, p, EffectKind::LoseAllAbilities, Duration::EndOfTurn);
    assert!(!banding::can_lead(&g, p));
}
#[test]
fn a_declared_band_keeps_its_members_after_ability_loss_and_blocks_as_a_group() {
    let mut g = board();
    let p = put(&mut g, &PROVIDER, A);
    let x = put(&mut g, &LEGEND, A);
    let y = put(&mut g, &LEGEND, A);
    let other = put(&mut g, &LEGEND, A);
    let blocker = put(&mut g, &PLAIN, B);
    g.run();
    let r = g.pending().cloned().unwrap();
    assert!(matches!(r.kind, RequestKind::DeclareAttackers { .. }));
    g.answer(
        r.id,
        A,
        Answer::Attack(vec![
            (p, Target::Player(B)),
            (x, Target::Player(B)),
            (y, Target::Player(B)),
            (other, Target::Player(C)),
        ]),
    )
    .unwrap();
    g.run();
    let r = g.pending().cloned().unwrap();
    let RequestKind::ChooseCards { cards, .. } = &r.kind else {
        panic!("{:?}", r.kind)
    };
    assert!(cards.contains(&x) && cards.contains(&y));
    assert!(!cards.contains(&other));
    g.answer(r.id, A, Answer::Cards(vec![x, y])).unwrap();
    g.run();
    let band = g.state.attackers.iter().find(|a| a.id == p).unwrap().band;
    assert_ne!(band, 0);
    assert!(
        g.state
            .attackers
            .iter()
            .filter(|a| [p, x, y].contains(&a.id))
            .all(|a| a.band == band)
    );
    cx(&mut g).add_effect(p, p, EffectKind::LoseBandsWithOthers, Duration::EndOfTurn);
    assert!(!banding::can_lead(&g, p));
    force_step(&mut g, Step::DeclareBlockers);
    g.run();
    let r = g.pending().cloned().unwrap();
    assert!(matches!(r.kind, RequestKind::DeclareBlockers { .. }));
    g.answer(r.id, B, Answer::Block(vec![(blocker, x)]))
        .unwrap();
    g.run();
    for id in [p, x, y] {
        assert!(
            g.state
                .blocks
                .iter()
                .any(|b| b.blocker == blocker && b.attacker == id)
        );
    }
    cx(&mut g).remove_from_combat(y);
    assert!(!g.state.attackers.iter().any(|a| a.id == y));
    assert!(
        g.state
            .attackers
            .iter()
            .filter(|a| [p, x].contains(&a.id))
            .all(|a| a.band == band)
    );
}
#[test]
fn the_matching_pairs_controller_assigns_damage_even_with_an_unrelated_member_and_no_declared_band()
{
    for attacking in [true, false] {
        let mut g = board();
        let owner = if attacking { B } else { A };
        let source = put(&mut g, &HUGE, if attacking { A } else { B });
        let p = put(&mut g, &PROVIDER, owner);
        let peer = put(&mut g, &LEGEND, owner);
        let out = put(&mut g, &PLAIN, owner);
        if attacking {
            g.state.set_attackers(vec![Attacker {
                blocked: true,
                ..Attacker::new(source, Target::Player(B), B, TypeSet::NONE)
            }]);
            g.state.set_blocks(
                [p, peer, out]
                    .into_iter()
                    .map(|blocker| Block {
                        blocker,
                        attacker: source,
                        first_strike: false,
                    })
                    .collect(),
            );
        } else {
            g.state.set_attackers(
                [p, peer, out]
                    .into_iter()
                    .map(|id| Attacker {
                        blocked: true,
                        ..Attacker::new(id, Target::Player(B), B, TypeSet::NONE)
                    })
                    .collect(),
            );
            g.state.set_blocks(
                [p, peer, out]
                    .into_iter()
                    .map(|attacker| Block {
                        blocker: source,
                        attacker,
                        first_strike: false,
                    })
                    .collect(),
            );
        }
        force_step(&mut g, Step::CombatDamage);
        g.run();
        let r = g.pending().cloned().unwrap();
        assert_eq!(r.player, owner);
        let RequestKind::AssignCombatDamage { divisions } = &r.kind else {
            panic!("{:?}", r.kind)
        };
        let d = divisions.iter().find(|d| d.source == source).unwrap();
        assert!(d.banding);
        assert!(!d.may_assign_to_defender);
        g.answer(r.id, owner, Answer::Shares(vec![(source, Some(out), 9)]))
            .unwrap();
        g.run();
        assert_eq!(g.state.player(B).life, 20);
        assert!(g.state.obj(out).became.is_some());
        assert!(g.state.obj(p).became.is_none());
        assert!(g.state.obj(peer).became.is_none());
    }
}
