//! The request in an observation (`cantrip_obs::Ask`): every constraint an answer must meet, in canonical terms. A
//! list the engine orders is sorted canonically, and where an answer names an entry by its place in such a list (an
//! option, a trigger, a replacement effect) the permutation back to the engine's is kept beside the ask (`Perms`),
//! never in it.

use std::collections::HashMap;

use cantrip_engine::*;
use cantrip_obs::*;

use crate::observe::target_ref;

/// The engine's index of each canonical entry: `options[i]` is the engine option the ask's `i`th option is; `order[i]`
/// the engine's trigger or replacement effect the ask's `i`th is.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Perms {
    pub options: Vec<usize>,
    pub order: Vec<usize>,
}

pub(crate) fn cost_obs(c: &ManaCost) -> CostObs {
    CostObs {
        generic: c.generic,
        symbols: c.symbols,
        hybrid: c
            .hybrid
            .iter()
            .flatten()
            .map(|&(a, b)| (a as u8, b as u8))
            .collect(),
        phyrexian: c.phyrexian,
        x: c.x,
    }
}

pub(crate) fn option_obs(o: &ActionOption, refs: &HashMap<ObjectId, u32>) -> OptionObs {
    let mut target_candidates: Vec<Ref> = o
        .target_candidates
        .iter()
        .map(|&t| target_ref(t, refs))
        .collect();
    target_candidates.sort();
    OptionObs {
        kind: o.kind as u8,
        source: o
            .source
            .map_or(Ref::None, |s| target_ref(Target::Object(s), refs)),
        index: o.index,
        targets_required: u8::try_from(o.targets_required).unwrap_or(u8::MAX),
        targets_optional: o.targets_optional,
        target_candidates,
        color_choices: o.color_choices.0,
        names_nothing: o.names_nothing,
        mana: o.mana.map(|m| m as u8),
        restriction: o.restriction.map(|r| format!("{r:?}")),
        pays_nothing: o.pays_nothing,
        granted_by: o.granted_by.map(|g| (target_ref(Target::Object(g.object), refs), g.ability.index)),
        paid_by: o.paid_by.map(|g| (target_ref(Target::Object(g.object), refs), g.ability.index)),
    }
}

/// Options sorted canonically, and the engine index of each.
fn options(
    os: &[ActionOption],
    refs: &HashMap<ObjectId, u32>,
    perms: &mut Perms,
) -> Vec<OptionObs> {
    let mut all: Vec<(OptionObs, usize)> = os
        .iter()
        .enumerate()
        .map(|(i, o)| (option_obs(o, refs), i))
        .collect();
    all.sort();
    perms.options = all.iter().map(|a| a.1).collect();
    all.into_iter().map(|a| a.0).collect()
}

/// A `Purpose` by name: its variant, and for a question its verb.
pub(crate) fn purpose_name(p: &Purpose) -> String {
    if let Purpose::Ask(q) = p {
        return format!("Ask:{}", q.verb);
    }
    let debug = format!("{p:?}");
    let end = debug.find(['{', '(', ' ']).unwrap_or(debug.len());
    debug[..end].to_string()
}

fn label_name(game: &Game, l: &Label) -> String {
    let name = |c: CardId| game.cards.get(c).name;
    match *l {
        Label::Card(c, n) => format!("Card:{}:{n}", name(c)),
        Label::Name(c) => format!("Name:{}", name(c)),
        Label::Mode(a, n) => format!("Mode:{}:{}:{n}", name(a.card), a.index),
        Label::Extra(c, n) => format!("Extra:{}:{n}", name(c)),
        Label::Face(c) => format!("Face:{}", name(c)),
        Label::Miracle(m) => format!("Miracle:{}:{}:{}", name(m.card), name(m.granter), m.index),
        ref other => format!("{other:?}"),
    }
}

/// Refs sorted, and a list carried with them index for index.
fn sorted_with<T: Clone>(refs: Vec<Ref>, with: &[T]) -> (Vec<Ref>, Vec<T>) {
    let mut both: Vec<(Ref, usize)> = refs.into_iter().enumerate().map(|(i, r)| (r, i)).collect();
    both.sort();
    let with = both.iter().map(|&(_, i)| with[i].clone()).collect();
    (both.into_iter().map(|b| b.0).collect(), with)
}

/// The most mana `player` could have as far as they can tell: their pool and, for each untapped permanent of theirs with
/// a mana ability, the most one of its mana abilities adds (one where it lists none). An estimate, for the X buckets a
/// policy may choose among: cost reductions and mana from elsewhere are not counted.
fn mana_available(game: &Game, player: PlayerId) -> u32 {
    let s = &game.state;
    // Restricted mana counts too (CR 106.6): the estimate may be over what can be spent on this cost.
    let pool: u32 = s
        .player(player)
        .pool
        .0
        .iter()
        .chain(
            s.restricted_mana
                .iter()
                .filter(|r| r.player == player)
                .flat_map(|r| r.pool.0.iter()),
        )
        .map(|&n| u32::from(n))
        .sum();
    let sources: u32 = s
        .zone(Zone::BATTLEFIELD)
        .iter()
        .filter(|&&id| s.obj(id).controller == player && !s.obj(id).tapped)
        .map(|&id| {
            game.chars(id)
                .abilities()
                .filter_map(|a| match game.cards.ability(a) {
                    AbilityDef::Activated(d) if d.is_mana_ability() => Some(
                        (d.adds.0.iter().map(|&n| u32::from(n)).sum::<u32>()
                            + u32::from(d.adds_any))
                        .max(1),
                    ),
                    _ => None,
                })
                .max()
                .unwrap_or(0)
        })
        .sum();
    pool + sources
}

/// The most of a running total of mana value a rule part keeps (`Tally::ManaValueAtMost`), if it keeps one.
fn tallied(part: &select::Part) -> Option<u32> {
    (part.tally == select::Tally::ManaValueAtMost).then_some(part.n)
}

/// A selection rule (`Selection`) in the ask's terms: the cards that may be chosen first (`first_choices`), the most that
/// may be chosen (every one of those for any number), whether fewer may be (stopping at none as the rule allows: a search
/// that fails to find, CR 701.19b), and the most of a running total of mana value where a part keeps one. The rest of the
/// rule - what a card already chosen rules out, "as many as can" - is the validator's: an answer it refuses is one the
/// caller backtracks from (`Rejection::global`).
fn selection(game: &Game, request: &Request, rule: &Selection) -> (Vec<Target>, u8, bool, Option<u32>) {
    let eligible: Vec<Target> = first_choices(game, request).into_iter().map(Target::Object).collect();
    let none_complete = selection_verdict(game, request, &[]).is_some_and(|(_, v)| v.complete);
    let (count, up_to) = match rule.count {
        cantrip_engine::Count::Exactly(n) => (n, none_complete),
        cantrip_engine::Count::UpTo(n) | cantrip_engine::Count::AsManyAsCan(n) => (n, true),
        cantrip_engine::Count::AnyNumber => (count_u32(eligible.len()), true),
    };
    let most = rule.parts().find_map(tallied);
    (eligible, u8::try_from(count).unwrap_or(u8::MAX), up_to, most)
}

/// What a name must be (`NameKind`) as the ask's number: 0 any, 1 nonland, 2 land, 3 any other kind.
fn name_kind(kind: NameKind) -> u8 {
    match kind {
        NameKind::ANY => 0,
        NameKind::NONLAND => 1,
        NameKind::LAND => 2,
        _ => 3,
    }
}

/// The ask of `request`, for its player, and the permutations back to the engine's lists.
pub(crate) fn ask_of(
    game: &Game,
    request: &Request,
    refs: &HashMap<ObjectId, u32>,
) -> (Ask, Perms) {
    let r = |id: ObjectId| target_ref(Target::Object(id), refs);
    let t = |x: Target| target_ref(x, refs);
    let rs = |ids: &[ObjectId]| -> Vec<Ref> {
        let mut v: Vec<Ref> = ids.iter().map(|&id| r(id)).collect();
        v.sort();
        v
    };
    // A rule that keeps a running total of mana value (CR 202.3; `Tally::ManaValueAtMost`): its most, and each eligible
    // card's mana value, index for index, read where it is (as the host's prompt reads it).
    let total_obs = |eligible: &[Target], most: Option<u32>| -> (Vec<Ref>, Option<TotalObs>) {
        let refs: Vec<Ref> = eligible.iter().map(|&e| t(e)).collect();
        match most {
            Some(most) => {
                let values: Vec<u16> = eligible
                    .iter()
                    .map(|e| match *e {
                        Target::Object(id) => u16::try_from(game.mana_value(id)).unwrap_or(u16::MAX),
                        _ => 0,
                    })
                    .collect();
                let (eligible, values) = sorted_with(refs, &values);
                (
                    eligible,
                    Some(TotalObs {
                        values,
                        most: u16::try_from(most).unwrap_or(u16::MAX),
                    }),
                )
            }
            None => {
                let mut e = refs;
                e.sort();
                (e, None)
            }
        }
    };
    let mut perms = Perms::default();
    let mut source = Ref::None;
    let kind = match &request.kind {
        RequestKind::Priority { options: os } => AskKind::Priority {
            options: options(os, refs, &mut perms),
        },
        RequestKind::PayCosts {
            source: s,
            total,
            remaining,
            options: os,
            announcing,
            x,
            most,
            loyalty,
            attackers,
            life,
            ..
        } => {
            source = r(*s);
            // The engine knows the largest X that may be announced (`most`), from the mana, loyalty or life the
            // cost allows; while paying it is zero and unused.
            let x_most = *most;
            AskKind::PayCosts {
                total: cost_obs(total),
                remaining: cost_obs(remaining),
                options: options(os, refs, &mut perms),
                announcing: *announcing,
                x: u16::try_from(*x).unwrap_or(u16::MAX),
                x_most: u16::try_from(x_most).unwrap_or(u16::MAX),
                loyalty: *loyalty,
                attackers: rs(attackers),
                life: *life,
            }
        }
        RequestKind::UnlessPays {
            source: s,
            subject,
            total,
            remaining,
            life,
            life_payable,
            question,
            options: os,
            x,
            ..
        } => {
            source = r(*s);
            let x_most = match x {
                // `total` is the cost at the X so far, its {X} as generic mana: what is left over pays more X.
                Some(n) => (mana_available(game, request.player).saturating_sub(total.mana_value())
                    + u32::from(*n))
                .min(u32::from(u16::MAX)) as u16,
                None => 0,
            };
            AskKind::UnlessPays {
                subject: r(*subject),
                total: cost_obs(total),
                remaining: cost_obs(remaining),
                life: u8::try_from(*life).unwrap_or(u8::MAX),
                life_payable: *life_payable,
                verb: question.verb.to_string(),
                options: options(os, refs, &mut perms),
                x: *x,
                x_most,
            }
        }
        RequestKind::ChooseCards {
            source: s,
            purpose,
            cards,
            rule,
        } => {
            source = r(*s);
            let (eligible, count, up_to, most) = selection(game, request, rule);
            let (eligible, total) = total_obs(&eligible, most);
            AskKind::ChooseCards {
                purpose: purpose_name(purpose),
                cards: rs(cards),
                eligible,
                count,
                up_to,
                total,
            }
        }
        RequestKind::ChooseTarget {
            source: s,
            purpose,
            candidates,
            optional,
            least,
            count,
            any_number,
            slot_player,
            together,
            ..
        } => {
            source = r(*s);
            // The rest of a rule of the targets together is the validator's: a refusal the caller backtracks from.
            let (candidates, total) = total_obs(candidates, together.and_then(|p| tallied(&p)));
            AskKind::ChooseTarget {
                purpose: purpose_name(purpose),
                candidates,
                optional: *optional,
                least: u8::try_from(*least).unwrap_or(u8::MAX),
                count: u8::try_from(*count).unwrap_or(u8::MAX),
                any_number: *any_number,
                slot_player: slot_player.map(|p| p.0),
                total,
            }
        }
        RequestKind::ChooseModes {
            source: s,
            modes,
            eligible,
            count,
            at_most,
            affordable,
            additional_cost,
            repeat,
        } => {
            source = r(*s);
            AskKind::ChooseModes {
                labels: modes.iter().map(|l| label_name(game, l)).collect(),
                eligible: eligible.clone(),
                count: *count,
                at_most: *at_most,
                affordable: affordable.clone(),
                additional_cost: *additional_cost,
                repeat: *repeat,
            }
        }
        RequestKind::ChooseColors {
            source: s,
            choices,
            count,
            purpose,
        } => {
            source = r(*s);
            AskKind::ChooseColors {
                choices: choices.0,
                count: *count,
                purpose: *purpose as u8,
            }
        }
        RequestKind::ChooseName {
            source: s,
            looked_at,
            suggestion,
            kind,
        } => {
            source = r(*s);
            AskKind::ChooseName {
                looked_at: rs(looked_at),
                suggestion: suggestion.map(|c| game.cards.get(c).name.to_string()),
                kind: name_kind(*kind),
            }
        }
        RequestKind::ChooseSubtype {
            source: s,
            choices,
            purpose,
        } => {
            source = r(*s);
            let mut choices: Vec<u16> = choices.iter().map(|&c| c as u16).collect();
            choices.sort_unstable();
            AskKind::ChooseSubtype {
                choices,
                purpose: *purpose as u8,
            }
        }
        RequestKind::ChooseNumber { source: s, least, most, .. } => {
            source = r(*s);
            let max_index = u8::try_from(most - least).expect("ChooseNumber range is bounded by the mechanic");
            AskKind::ChooseModes {
                labels: (*least..=*most).map(|n| n.to_string()).collect(),
                eligible: (0..=max_index).collect(),
                count: 1,
                at_most: 1,
                affordable: Vec::new(),
                additional_cost: false,
                repeat: false,
            }
        }
        RequestKind::May { source: s, purpose } => {
            source = r(*s);
            let verb = match purpose {
                Purpose::Ask(q) => q.verb.to_string(),
                _ => String::new(),
            };
            AskKind::May {
                purpose: purpose_name(purpose),
                verb,
            }
        }
        RequestKind::ChoosePlayer {
            source: s,
            candidates,
            verb,
            amount,
            ..
        } => {
            source = r(*s);
            let mut candidates: Vec<u8> = candidates.iter().map(|p| p.0).collect();
            candidates.sort_unstable();
            AskKind::ChoosePlayer {
                candidates,
                verb: verb.to_string(),
                amount: *amount,
            }
        }
        RequestKind::TapForPower {
            source: s,
            purpose,
            cards,
            powers,
            minimum,
            ..
        } => {
            source = r(*s);
            let (cards, powers) = sorted_with(cards.iter().map(|&id| r(id)).collect(), powers);
            AskKind::TapForPower {
                purpose: purpose_name(purpose),
                cards,
                powers,
                minimum: *minimum,
            }
        }
        RequestKind::CollectEvidence {
            source: s,
            cards,
            values,
            minimum,
        } => {
            source = r(*s);
            let (cards, values) = sorted_with(cards.iter().map(|&id| r(id)).collect(), values);
            AskKind::CollectEvidence {
                cards,
                values,
                minimum: *minimum,
            }
        }
        RequestKind::Craft {
            source: s,
            cards,
            minimum,
            maximum,
            ..
        } => {
            source = r(*s);
            AskKind::Craft {
                cards: rs(cards),
                minimum: u8::try_from(*minimum).unwrap_or(u8::MAX),
                maximum: u8::try_from(*maximum).unwrap_or(u8::MAX),
            }
        }
        RequestKind::RemoveCounters {
            source: s,
            pools,
            total,
            up_to,
        } => {
            source = r(*s);
            let mut pools: Vec<CounterPoolObs> = pools
                .iter()
                .map(|p| CounterPoolObs {
                    object: r(p.object),
                    kind: p.kind as u16,
                    count: p.count,
                })
                .collect();
            pools.sort_by_key(|p| (p.object, p.kind, p.count));
            AskKind::RemoveCounters {
                pools,
                total: *total,
                up_to: *up_to,
            }
        }
        RequestKind::ChooseDividedTargets {
            source: s,
            candidates,
            total,
            max_targets,
            optional,
        } => {
            source = r(*s);
            let mut candidates: Vec<Ref> = candidates.iter().map(|&c| t(c)).collect();
            candidates.sort();
            AskKind::ChooseDividedTargets {
                candidates,
                total: u16::try_from(*total).unwrap_or(u16::MAX),
                max_targets: u8::try_from(*max_targets).unwrap_or(u8::MAX),
                optional: *optional,
            }
        }
        RequestKind::DeclareAttackers {
            eligible,
            players,
            planeswalkers,
            battles,
            requirements,
            costs,
            can_pay,
        } => {
            // The most creatures that may attack each defender (CR 508.1c), as `AttackRules::limits` has them: u8::MAX where
            // nothing caps it. A cap of 255 or more reads as u8::MAX, "no cap", for the AI's mask only; the validator still
            // judges the declaration against the real cap.
            let cap = |at: Target| {
                requirements
                    .limits
                    .iter()
                    .find(|l| l.0 == at)
                    .map_or(u8::MAX, |l| u8::try_from(l.1).unwrap_or(u8::MAX))
            };
            let mut capped: Vec<(Ref, u8)> = players
                .iter()
                .map(|&p| (Ref::Player(p.0), cap(Target::Player(p))))
                .chain(planeswalkers.iter().chain(battles).map(|&id| (r(id), cap(Target::Object(id)))))
                .collect();
            capped.sort();
            let defenders: Vec<Ref> = capped.iter().map(|d| d.0).collect();
            let max_attackers: Vec<u8> = capped.iter().map(|d| d.1).collect();
            // What each attack obeys of the requirements (CR 508.1d), by the engine's own count.
            let targets: Vec<Target> = players
                .iter()
                .map(|&p| Target::Player(p))
                .chain(planeswalkers.iter().chain(battles).map(|&id| Target::Object(id)))
                .collect();
            let mut obeyed: Vec<(Ref, Ref, u32)> = eligible
                .iter()
                .flat_map(|&a| targets.iter().map(move |&d| (a, d)))
                .filter_map(|(a, d)| {
                    let n = requirements.obeyed(&[(a, d)]);
                    (n > 0).then(|| (r(a), t(d), n))
                })
                .collect();
            obeyed.sort();
            let mut costs: Vec<(Ref, Ref, u16)> = costs
                .iter()
                .map(|c| {
                    (
                        r(c.attacker),
                        t(c.target),
                        c.cost.mana_value().min(u32::from(u16::MAX)) as u16,
                    )
                })
                .collect();
            costs.sort();
            // CR 508.1c: the attacks a restriction forbids.
            let mut barred: Vec<(Ref, Ref)> = requirements.barred.iter().map(|&(a, d)| (r(a), t(d))).collect();
            barred.sort();
            AskKind::DeclareAttackers {
                eligible: rs(eligible),
                defenders,
                requirements: obeyed,
                most: requirements.most,
                costs,
                can_pay: *can_pay,
                max_attackers,
                barred,
            }
        }
        RequestKind::AssignCombatDamage { divisions } => {
            let mut divisions: Vec<DivisionObs> = divisions
                .iter()
                .map(|d| {
                    let (creatures, lethal) =
                        sorted_with(d.creatures.iter().map(|&id| r(id)).collect(), &d.lethal);
                    DivisionObs {
                        source: r(d.source),
                        banding: d.banding,
                        damage: d.damage,
                        creatures,
                        lethal,
                        may_assign_to_defender: d.may_assign_to_defender,
                        defender: t(d.defender),
                    }
                })
                .collect();
            divisions.sort_by_key(|d| d.source);
            AskKind::AssignCombatDamage { divisions }
        }
        RequestKind::DeclareBlockers {
            eligible,
            attackers,
            allowed,
            requirements,
        } => {
            // The fewest and most creatures that may block each attacker (CR 702.111b, 509.1b), as the host's prompt reads them.
            let mut limits: Vec<(Ref, u8, u8)> = attackers
                .iter()
                .map(|&a| {
                    let (least, most) = requirements.limits_of(a);
                    (r(a), u8::try_from(least).unwrap_or(u8::MAX), u8::try_from(most).unwrap_or(u8::MAX))
                })
                .collect();
            limits.sort();
            let attackers: Vec<Ref> = limits.iter().map(|l| l.0).collect();
            let min_blockers = limits.iter().map(|l| l.1).collect();
            let max_blockers = limits.iter().map(|l| l.2).collect();
            let pairs = |ps: &[(ObjectId, ObjectId)]| -> Vec<(Ref, Ref)> {
                let mut v: Vec<(Ref, Ref)> = ps.iter().map(|&(b, a)| (r(b), r(a))).collect();
                v.sort();
                v.dedup();
                v
            };
            // Each requirement as the allowed blocks that obey it (CR 509.1c): "blocks if able" is any attacker it may block,
            // "must be blocked if able" any creature that may block it.
            let required: Vec<(ObjectId, ObjectId)> = requirements
                .required
                .iter()
                .flat_map(|q| allowed.iter().copied().filter(move |&(b, a)| q.blocker.is_none_or(|x| x == b) && q.attacker.is_none_or(|x| x == a)))
                .collect();
            AskKind::DeclareBlockers {
                eligible: rs(eligible),
                attackers,
                allowed: pairs(allowed),
                required: pairs(&required),
                most: u8::try_from(requirements.most).unwrap_or(u8::MAX),
                min_blockers,
                max_blockers,
            }
        }
        RequestKind::DiscardToHandSize { count, hand } => AskKind::DiscardToHandSize {
            count: u8::try_from(*count).unwrap_or(u8::MAX),
            hand: rs(hand),
        },
        RequestKind::Mulligan { mulligans } => AskKind::Mulligan {
            mulligans: *mulligans,
        },
        RequestKind::ChooseStartingPlayer { candidates } => {
            let mut candidates: Vec<u8> = candidates.iter().map(|p| p.0).collect();
            candidates.sort_unstable();
            AskKind::ChooseStartingPlayer { candidates }
        }
        RequestKind::OrderTriggers { sources, abilities } => {
            let mut all: Vec<((Ref, String, u8), usize)> = sources
                .iter()
                .zip(abilities)
                .enumerate()
                .map(|(i, (&s, a))| ((r(s), game.cards.get(a.card).name.to_string(), a.index), i))
                .collect();
            all.sort();
            perms.order = all.iter().map(|a| a.1).collect();
            AskKind::OrderTriggers {
                sources: all.iter().map(|a| a.0.0).collect(),
                abilities: all.iter().map(|a| a.0.2).collect(),
            }
        }
        RequestKind::DistributeCards {
            source: s,
            cards,
            destinations,
            may_shuffle,
        } => {
            source = r(*s);
            AskKind::DistributeCards {
                cards: cards.iter().map(|&id| r(id)).collect(),
                destinations: destinations
                    .iter()
                    .map(|d| DestinationObs {
                        kind: d.kind as u8,
                        min: d.min,
                        max: d.max,
                        eligible: cards.iter().map(|c| d.eligible.contains(c)).collect(),
                    })
                    .collect(),
                may_shuffle: *may_shuffle,
            }
        }
        RequestKind::SearchLibrary {
            source: s,
            library,
            rule,
            destination,
        } => {
            source = r(*s);
            let (eligible, count, optional, _) = selection(game, request, rule);
            let mut eligible: Vec<Ref> = eligible.into_iter().map(t).collect();
            eligible.sort();
            AskKind::SearchLibrary {
                library: rs(library),
                eligible,
                count,
                optional,
                destination: *destination as u8,
            }
        }
        RequestKind::ChooseReplacement {
            subject,
            life,
            sources,
            hints,
            damage,
            turn,
            counters,
            draw,
        } => {
            let mut all: Vec<((Ref, String), usize)> = sources
                .iter()
                .enumerate()
                .map(|(i, &s)| {
                    (
                        (
                            r(s),
                            hints.get(i).map_or_else(String::new, |h| h.to_string()),
                        ),
                        i,
                    )
                })
                .collect();
            all.sort();
            perms.order = all.iter().map(|a| a.1).collect();
            AskKind::ChooseReplacement {
                subject: subject.map_or(Ref::None, r),
                life: *life,
                sources: all.iter().map(|a| a.0.0).collect(),
                hints: all.iter().map(|a| a.0.1.clone()).collect(),
                damage: damage.map(|d| (r(d.source), t(d.to), d.amount)),
                turn: *turn,
                counters: counters.iter().map(|&(k, n)| (k as u16, n)).collect(),
                draw: *draw,
            }
        }
    };
    (
        Ask {
            may_cancel: request.may_cancel,
            source,
            kind,
        },
        perms,
    )
}
