//! Judging a declaration of attackers (CR 508.1a-d): one validator for every restriction, requirement and cost a
//! declaration answers to, from the rules the question holds (`AttackRules`, beside the attackers that may attack, what they
//! may attack and what attacks cost). `AttackRules::judge` says whether a declaration may be made as it is
//! (`AttackVerdict::complete`), why not (`AttackVerdict::issue`), what it costs, and which attacks may be added to it on the
//! way to one that may (`AttackVerdict::addable`). The core checks every answer with it, the scenario harness reads it, and
//! the host gives its verdict to a client as its player declares (`CheckAttacks`), so no client works CR 508.1d out.
//!
//! CR 508.1d counts requirements: the declaration must obey as many as any declaration that obeys every restriction can
//! without paying a cost. A goaded creature (CR 701.15b) is under two for each player who goaded it - attack, and attack a
//! player other than that one - and "attacks that player this combat if able" is one: attacking either of two players may
//! obey as many as the other, and both are legal. A restriction on one attacker at a time (who may attack, and what) leaves
//! the most each attacker's best attack, added up; a cap on how many may attack one player, planeswalker or battle (CR
//! 508.1c) is room the attackers share, which makes the most a maximum-weight assignment with capacities (`Assignment`), solved
//! exactly in polynomial time.

use std::collections::HashMap;
#[cfg(test)]
thread_local! { pub(crate) static GROUP_ATTACK_SEARCH_NODES: std::cell::Cell<usize> = const { std::cell::Cell::new(0) }; }

use crate::{AttackCost, ManaCost, ObjectId, PlayerId, Target, count_u32};

/// What a requirement on a declaration of attackers asks of its attacker (CR 508.1d).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AttackAim {
    /// That it attack, whatever it attacks: "attacks each combat if able", a goad's first requirement (CR 701.15b).
    Anything,
    /// That it attack this player, planeswalker or battle: "attacks that player this combat if able".
    At(Target),
    /// That it attack a player other than this one: a goad's second requirement, the goader's (CR 701.15b).
    PlayerOtherThan(PlayerId),
}

/// One requirement on a declaration of attackers (CR 508.1d), one for each effect, ability or player it comes from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct RequiredAttack {
    pub attacker: ObjectId,
    pub aim: AttackAim,
}

/// The rules of a declaration of attackers past which attacks are allowed (CR 508.1c-d), held in the question so the
/// game-log tree moves through them: the requirements; the attacks a restriction forbids one creature to make; the most
/// creatures that may attack each player, planeswalker or battle, where something caps it; the most of them any declaration that obeys every restriction obeys without paying a
/// cost, which the declaration must obey too; and one such declaration, the engine's suggestion.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct AttackRules {
    pub required: Vec<RequiredAttack>,
    /// The attacks of an eligible creature at a player, planeswalker or battle it may otherwise attack that a restriction
    /// forbids (CR 508.1c): "creatures with flying can't attack you" (Storm, Windrider) bars a flier's attack on that player
    /// and no other. A creature forbidden every attack is not eligible at all.
    pub barred: Vec<(ObjectId, Target)>,
    /// The creatures that a static ability forbids to attack alone (CR 508.1c): a declaration whose only attacker is one
    /// of these is illegal. Pet Sehlat's "This creature can't attack alone".
    pub alone_barred: Vec<ObjectId>,
    /// The most creatures that may attack each capped player, planeswalker or battle (CR 508.1c), one entry each.
    pub limits: Vec<(Target, u32)>,
    /// Whole-declaration count restrictions (CR 508.1c), empty in ordinary games.
    pub groups: Vec<crate::CombatGroup>,
    pub most: u32,
    pub suggested: Vec<(ObjectId, Target)>,
}

/// What the rules make of a declaration of attackers.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct AttackVerdict {
    /// It may be declared as it is (CR 508.1a-d); its cost, if any, is paid once it is declared (CR 508.1h-j).
    pub complete: bool,
    /// The attacks, by creatures not attacking, that some legal declaration extending it has (empty unless asked for).
    pub addable: Vec<(ObjectId, Target)>,
    /// Why it may not be declared as it is; none when it may.
    pub issue: Option<AttackIssue>,
    /// What declaring it costs (CR 508.1h), zero when nothing.
    pub cost: ManaCost,
    /// Nothing is required of the declaration (the most is 0), nothing caps how many may attack a target and nothing forbids
    /// a creature to attack alone: any
    /// declaration of `addable` attacks, each creature attacking once, is legal, so a client need not ask about each one.
    pub free: bool,
}

/// Why a declaration of attackers may not be made as it is, as a caption reads it: `creature`, then `words`, then `other`
/// where there is one ("Ursine Monstrosity" "must attack" Bot2; "Grizzly Bears" "must attack a player other than" Bot2), with
/// the rule it breaks.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct AttackIssue {
    pub creature: ObjectId,
    pub words: String,
    pub other: Option<Target>,
    pub rule: &'static str,
}

impl AttackRules {
    /// Whether a restriction forbids `attacker` to attack `at` (CR 508.1c).
    pub fn bars(&self, attacker: ObjectId, at: Target) -> bool {
        self.barred.contains(&(attacker, at))
    }

    /// Whether `attacker` attacking `at` obeys `r`.
    pub fn obeys(r: &RequiredAttack, attacker: ObjectId, at: Target) -> bool {
        r.attacker == attacker
            && match r.aim {
                AttackAim::Anything => true,
                AttackAim::At(t) => t == at,
                AttackAim::PlayerOtherThan(p) => matches!(at, Target::Player(q) if q != p),
            }
    }

    /// How many requirements `attacker` attacking `at` obeys: none where that attack is forbidden (CR 508.1c-d).
    fn gain(&self, attacker: ObjectId, at: Target) -> u32 {
        if self.bars(attacker, at) {
            return 0;
        }
        count_u32(self.required.iter().filter(|r| Self::obeys(r, attacker, at)).count())
    }

    /// How many creatures may attack `at` (CR 508.1c), or `None` when nothing caps it.
    fn limit(&self, at: Target) -> Option<u32> {
        self.limits.iter().find(|&&(t, _)| t == at).map(|&(_, n)| n)
    }

    /// Whether `at`'s cap leaves room for one more attack among `attacks`.
    fn room(&self, at: Target, attacks: &[(ObjectId, Target)]) -> bool {
        self.limit(at).is_none_or(|n| count_u32(attacks.iter().filter(|x| x.1 == at).count()) < n)
    }

    /// How many of the requirements a declaration obeys (CR 508.1d).
    pub fn obeyed(&self, attacks: &[(ObjectId, Target)]) -> u32 {
        attacks.iter().map(|&(a, t)| self.gain(a, t)).sum()
    }

    /// What each of `creatures` obeys attacking each of `targets`, by index (CR 508.1d): the requirements read once, however
    /// many creatures and attacks are asked about; nothing for an attack a restriction forbids (CR 508.1c), which obeys
    /// no requirement because it can't be made.
    fn gains(&self, creatures: &[ObjectId], targets: &[Target]) -> Vec<Vec<u32>> {
        let mut table = vec![vec![0u32; targets.len()]; creatures.len()];
        if self.required.is_empty() {
            return table;
        }
        let index: HashMap<ObjectId, usize> = creatures.iter().enumerate().map(|(i, &a)| (a, i)).collect();
        for r in &self.required {
            if let Some(&i) = index.get(&r.attacker) {
                for (t, &at) in targets.iter().enumerate() {
                    if Self::obeys(r, r.attacker, at) && !self.bars(r.attacker, at) {
                        table[i][t] += 1;
                    }
                }
            }
        }
        table
    }

    /// Whether a cap applies to any of `targets` (CR 508.1c).
    fn capped(&self, targets: &[Target]) -> bool {
        self.limits.iter().any(|l| targets.contains(&l.0))
    }

    /// The most requirements any declaration of `eligible` attacking among `targets` obeys without paying a cost (CR
    /// 508.1d), with one that does (`Assignment`).
    pub fn solve(&self, eligible: &[ObjectId], targets: &[Target], costs: &[AttackCost]) -> (u32, Vec<(ObjectId, Target)>) {
        if !self.groups.is_empty() {
            return self.solve_groups(eligible, targets, costs, &[], None, true).unwrap_or_default();
        }
        let gains = self.gains(eligible, targets);
        let rows: Vec<&[u32]> = gains.iter().map(|r| &r[..]).collect();
        let Some(mut best) = Assignment::new(self, &rows, eligible, targets, costs, true, &[]) else { return Default::default() };
        best.maximize();
        let declaration = best.declaration(eligible, targets);
        match declaration[..] {
            [(a, t)] if self.alone_barred.contains(&a) => self.not_alone(eligible, targets, costs, &gains, best.total(), a, t),
            _ => (best.total(), declaration),
        }
    }

    /// Exact branch-and-bound only for live group restrictions. Ordinary games
    /// keep the polynomial assignment solver. Bounds count each remaining
    /// creature's best attack; group maxima and impossible minima prune early.
    fn solve_groups(
        &self,
        eligible: &[ObjectId],
        targets: &[Target],
        costs: &[AttackCost],
        fixed: &[(ObjectId, Target)],
        enough: Option<u32>,
        free_only: bool,
    ) -> Option<(u32, Vec<(ObjectId, Target)>)> {
        for (i, &(a, t)) in fixed.iter().enumerate() {
            if !eligible.contains(&a)
                || !targets.contains(&t)
                || self.bars(a, t)
                || fixed[..i].iter().any(|x| x.0 == a)
                || (free_only && !AttackCost::free(costs, a, t))
            {
                return None;
            }
        }
        if self.limits.iter().any(|&(t, n)| count_u32(fixed.iter().filter(|x| x.1 == t).count()) > n) {
            return None;
        }
        let mut options: Vec<_> = eligible
            .iter()
            .copied()
            .filter(|a| !fixed.iter().any(|x| x.0 == *a))
            .map(|a| {
                let mut choices: Vec<_> = targets
                    .iter()
                    .copied()
                    .filter(|&t| !self.bars(a, t) && (!free_only || AttackCost::free(costs, a, t)))
                    .map(|t| (t, self.gain(a, t)))
                    .collect();
                choices.sort_by_key(|x| std::cmp::Reverse(x.1));
                (a, choices)
            })
            .collect();
        options.sort_by_key(|x| std::cmp::Reverse(x.1.first().map_or(0, |p| p.1)));
        let mut after = vec![0; options.len() + 1];
        for i in (0..options.len()).rev() {
            after[i] = after[i + 1] + options[i].1.first().map_or(0, |p| p.1);
        }
        struct Search<'a> {
            rules: &'a AttackRules,
            options: &'a [(ObjectId, Vec<(Target, u32)>)],
            after: &'a [u32],
            enough: Option<u32>,
            chosen: Vec<(ObjectId, Target)>,
            best: Option<(u32, Vec<(ObjectId, Target)>)>,
        }
        impl Search<'_> {
            fn done(&self) -> bool {
                self.enough.is_some_and(|n| self.best.as_ref().is_some_and(|b| b.0 >= n))
            }
            fn go(&mut self, i: usize, gain: u32) {
                #[cfg(test)]
                GROUP_ATTACK_SEARCH_NODES.with(|n| n.set(n.get() + 1));
                let has = |id| self.chosen.iter().any(|p| p.0 == id);
                for g in &self.rules.groups {
                    if !g.subjects.iter().any(|&id| has(id)) {
                        continue;
                    }
                    let count = g.count(has);
                    let more = g.members.iter().filter(|&&id| !has(id) && self.options[i..].iter().any(|p| p.0 == id && !p.1.is_empty())).count();
                    if count > g.at_most || count + count_u32(more) < g.at_least {
                        return;
                    }
                }
                let valid = self.rules.groups.iter().all(|g| g.allows(has))
                    && !(self.chosen.len() == 1 && self.rules.alone_barred.contains(&self.chosen[0].0));
                if valid && self.best.as_ref().is_none_or(|b| gain > b.0) && self.enough.is_none_or(|n| gain >= n) {
                    self.best = Some((gain, self.chosen.clone()));
                }
                if self.done() || i == self.options.len() {
                    return;
                }
                let mut remaining = self.after[i];
                for g in &self.rules.groups {
                    // Once a subject is chosen, its maximum also bounds the
                    // remaining requirement gain. A global cap can bound the
                    // root too: every possible participant is a subject.
                    if !g.subjects.iter().any(|&id| has(id)) && !self.options[i..].iter().all(|p| g.subjects.contains(&p.0)) {
                        continue;
                    }
                    let mut room = g.at_most.saturating_sub(g.count(has));
                    let mut bound = 0;
                    for (id, choices) in &self.options[i..] {
                        if !g.members.contains(id) {
                            bound += choices.first().map_or(0, |p| p.1);
                        } else if room > 0 {
                            room -= 1;
                            bound += choices.first().map_or(0, |p| p.1);
                        }
                    }
                    remaining = remaining.min(bound);
                }
                let reach = gain + remaining;
                if self.enough.is_some_and(|n| reach < n) || self.best.as_ref().is_some_and(|b| reach <= b.0) {
                    return;
                }
                let (a, choices) = &self.options[i];
                for &(t, g) in choices {
                    if !self.rules.room(t, &self.chosen) {
                        continue;
                    }
                    self.chosen.push((*a, t));
                    self.go(i + 1, gain + g);
                    self.chosen.pop();
                    if self.done() {
                        return;
                    }
                }
                self.go(i + 1, gain);
            }
        }
        let gain = self.obeyed(fixed);
        let mut search = Search { rules: self, options: &options, after: &after, enough, chosen: fixed.to_vec(), best: None };
        search.go(0, gain);
        search.best
    }

    fn judge_groups(
        &self,
        eligible: &[ObjectId],
        targets: &[Target],
        costs: &[AttackCost],
        attacks: &[(ObjectId, Target)],
        addable: bool,
    ) -> AttackVerdict {
        let mut base = self.clone();
        base.groups.clear();
        // Base restrictions remain authoritative; group requirements need the
        // exact group solver, rather than the ordinary assignment relaxation.
        base.most = 0;
        let mut verdict = base.judge(eligible, targets, costs, attacks, false);
        let has = |id| attacks.iter().any(|p| p.0 == id);
        let broken = self.groups.iter().find(|g| !g.allows(has));
        if verdict.issue.is_none() {
            if let Some(g) = broken {
                verdict.issue = Some(AttackIssue {
                    creature: g.subjects.iter().copied().find(|&id| has(id)).unwrap_or(ObjectId::NONE),
                    words: "can't attack with this group".into(),
                    other: None,
                    rule: "508.1c",
                });
            } else if self.obeyed(attacks) < self.most {
                verdict.issue = Some(AttackIssue {
                    creature: attacks.first().map_or(ObjectId::NONE, |p| p.0),
                    words: "must satisfy more attack requirements".into(),
                    other: None,
                    rule: "508.1d",
                });
            }
        }
        verdict.complete = verdict.issue.is_none();
        verdict.free = false;
        if addable {
            for &a in eligible.iter().filter(|&&a| !has(a)) {
                for &t in targets {
                    let mut fixed = attacks.to_vec();
                    fixed.push((a, t));
                    if self.solve_groups(eligible, targets, costs, &fixed, Some(self.most), false).is_some() {
                        verdict.addable.push((a, t));
                    }
                }
            }
        }
        verdict
    }

    /// The most requirements a declaration obeys without paying a cost (CR 508.1d) where the best one of all, obeying
    /// `most`, is `a` attacking `t` alone, which a static ability forbids (CR 508.1c: Pet Sehlat's "can't attack alone"): every
    /// other declaration is allowed that one is not. Each attack in the best declaration obeys something, so nothing else
    /// adds to it. Another creature attacking anything beside `a` for free obeys no fewer. Where none may, each other
    /// creature may attack for free only `t`, in `a`'s place (a cap of one there): the best is one of them there - beside
    /// `a` attacking elsewhere, or alone where it may attack alone - or no attack at all, which obeys none.
    #[allow(clippy::too_many_arguments)]
    fn not_alone(&self, eligible: &[ObjectId], targets: &[Target], costs: &[AttackCost], gains: &[Vec<u32>], most: u32, a: ObjectId, t: Target) -> (u32, Vec<(ObjectId, Target)>) {
        let free = |c: ObjectId, u: Target| !self.bars(c, u) && AttackCost::free(costs, c, u);
        if let Some((c, u)) = eligible
            .iter()
            .filter(|&&c| c != a)
            .find_map(|&c| targets.iter().find(|&&u| free(c, u) && self.room(u, &[(a, t)])).map(|&u| (c, u)))
        {
            return (most, vec![(a, t), (c, u)]);
        }
        let (Some(ai), Some(ti)) = (eligible.iter().position(|&c| c == a), targets.iter().position(|&u| u == t)) else { return Default::default() };
        let mut best: (u32, Vec<(ObjectId, Target)>) = (0, Vec::new());
        for (ci, &c) in eligible.iter().enumerate().filter(|&(_, &c)| c != a && free(c, t)) {
            let there = gains[ci][ti];
            if !self.alone_barred.contains(&c) && there > best.0 {
                best = (there, vec![(c, t)]);
            }
            for (ui, &u) in targets.iter().enumerate().filter(|&(_, &u)| u != t && free(a, u) && self.room(u, &[])) {
                if gains[ai][ui] + there > best.0 {
                    best = (gains[ai][ui] + there, vec![(a, u), (c, t)]);
                }
            }
        }
        best
    }

    /// What the rules make of `attacks`, a declaration of `eligible` creatures attacking among `targets`, with what attacks
    /// cost (CR 508.1a-d); with `addable`, which attacks may be added on the way to a legal one too.
    pub fn judge(&self, eligible: &[ObjectId], targets: &[Target], costs: &[AttackCost], attacks: &[(ObjectId, Target)], addable: bool) -> AttackVerdict {
        if !self.groups.is_empty() { return self.judge_groups(eligible, targets, costs, attacks, addable); }
        let issue = |creature, words: &str, other, rule| Some(AttackIssue { creature, words: words.to_string(), other, rule });
        let mut found = None;
        let mut mendable = true;
        for (i, &(a, t)) in attacks.iter().enumerate() {
            if attacks[..i].iter().any(|x| x.0 == a) {
                found = issue(a, "can attack only once", None, "508.1a");
            } else if !eligible.contains(&a) || !targets.contains(&t) || self.bars(a, t) {
                found = issue(a, "can't attack", Some(t), "508.1c");
            }
            if found.is_some() {
                mendable = false;
                break;
            }
        }
        // CR 508.1c: more creatures declared attacking one target than its cap allows is illegal, though an effect may
        // still put another there once they are declared. Nothing extends a declaration already over a cap, so none is
        // addable.
        if found.is_none() {
            for &(t, n) in &self.limits {
                let mut attacking = attacks.iter().filter(|x| x.1 == t).map(|x| x.0);
                if let Some(creature) = attacking.nth(n as usize) {
                    found = issue(creature, "can't attack", Some(t), "508.1c");
                    mendable = false;
                    break;
                }
            }
        }
        // CR 508.1c: one creature declared attacking alone that a static ability forbids to attack alone is illegal, and
        // another attacker beside it mends that.
        if found.is_none() && attacks.len() == 1 && self.alone_barred.contains(&attacks[0].0) {
            found = issue(attacks[0].0, "can't attack alone", None, "508.1c");
        }
        let declared = self.obeyed(attacks);
        let short = found.is_none() && declared < self.most;
        // CR 508.1c: an attack added to nothing by a creature a static ability forbids to attack alone is legal only with
        // another attacker beside it, so some other creature must be able to attack something with room left. Where the
        // best declaration with it has another attacker, that one can; where it has none, the others obey nothing, and one
        // of them attacking anything loses nothing.
        let lone = |a: ObjectId, at: Target| {
            attacks.is_empty()
                && self.alone_barred.contains(&a)
                && !eligible.iter().any(|&d| d != a && targets.iter().any(|&v| !self.bars(d, v) && self.room(v, &[(a, at)])))
        };
        let gains = if short || (addable && mendable) { self.gains(eligible, targets) } else { Vec::new() };
        if short {
            // An attacker obeying fewer than it could, and a requirement its best attack obeys that its attack does not.
            found = eligible
                .iter()
                .zip(&gains)
                .find_map(|(&a, row)| {
                    let now = attacks.iter().find(|x| x.0 == a).map(|x| x.1);
                    let (best, at) = best_of(row, a, targets, costs, true);
                    let have = now.map_or(0, |t| self.gain(a, t));
                    let at = targets[at?];
                    (have < best).then(|| {
                        let r = self.required.iter().find(|r| Self::obeys(r, a, at) && now.is_none_or(|t| !Self::obeys(r, a, t)))?;
                        match r.aim {
                            AttackAim::Anything => issue(a, "must attack", None, "508.1d"),
                            AttackAim::At(t) => issue(a, "must attack", Some(t), "508.1d"),
                            AttackAim::PlayerOtherThan(p) => issue(a, "must attack a player other than", Some(Target::Player(p)), "508.1d"),
                        }
                    })?
                })
                // Fewer obeyed than the most always leaves some attacker short of its best; should none be found, the
                // declaration is still refused, never complete by accident.
                .or_else(|| issue(attacks.first().map_or(ObjectId::NONE, |x| x.0), "must attack", None, "508.1d"));
        }
        let complete = found.is_none();
        let free: Vec<usize> = if addable && mendable { (0..eligible.len()).filter(|&i| !attacks.iter().any(|x| x.0 == eligible[i])).collect() } else { Vec::new() };
        let addable = if addable && mendable && !self.capped(targets) {
            // Every restriction is on one attacker at a time: an attack may be added where the others not yet attacking can
            // make up the rest at their best, paying if need be.
            let rest: Vec<(usize, u32)> = free.iter().map(|&i| (i, best_of(&gains[i], eligible[i], targets, costs, false).0)).collect();
            let all: u32 = rest.iter().map(|r| r.1).sum();
            let mut list = Vec::new();
            for &(i, best) in &rest {
                for (t, &at) in targets.iter().enumerate() {
                    if !self.bars(eligible[i], at) && !lone(eligible[i], at) && declared + gains[i][t] + all - best >= self.most {
                        list.push((eligible[i], at));
                    }
                }
            }
            list
        } else if addable && mendable {
            // A cap is room several creatures share (CR 508.1c), so an attack may be added where that target has room and
            // some declaration extending this one with it still obeys the most (CR 508.1d), paying if need be. One best
            // assignment of the creatures not attacking answers it for every attack: taking a creature out of it frees
            // its place, which goes where it adds the most (`fill`), and placing it at a full capped target makes room
            // there the way that loses the least (`shed`) - each the best change for one place more or less, so the
            // result is the best declaration with that attack, exactly.
            let creatures: Vec<ObjectId> = free.iter().map(|&i| eligible[i]).collect();
            let rows: Vec<&[u32]> = free.iter().map(|&i| &gains[i][..]).collect();
            let mut list = Vec::new();
            if let Some(mut best) = Assignment::new(self, &rows, &creatures, targets, costs, false, attacks) {
                best.maximize();
                let total = i64::from(best.total());
                let kept = best.at.clone();
                let room: Vec<bool> = targets.iter().map(|&t| self.room(t, attacks)).collect();
                for (i, &a) in creatures.iter().enumerate() {
                    let mut without = total - i64::from(best.value(i));
                    if let Some(j) = best.at[i].take() {
                        without += best.fill(j, i);
                    }
                    let shed = best.shed(i);
                    let loads = best.loads();
                    for (t, &at) in targets.iter().enumerate() {
                        if !room[t] || self.bars(a, at) || lone(a, at) {
                            continue;
                        }
                        let lose = match best.capped.iter().position(|c| c.0 == t) {
                            Some(j) if loads[j] >= best.capped[j].1 => match shed[j] {
                                Some(lose) => lose,
                                None => continue,
                            },
                            _ => 0,
                        };
                        if i64::from(declared) + i64::from(rows[i][t]) + without + lose >= i64::from(self.most) {
                            list.push((a, at));
                        }
                    }
                    best.at.copy_from_slice(&kept);
                }
            }
            list
        } else {
            Vec::new()
        };
        // Two attacks of `addable` may still be one too many for a capped target (CR 508.1c), and one of them alone may be an
        // attack a static ability forbids alone, so neither declaration is free of asking.
        let free = self.most == 0 && !self.capped(targets) && self.alone_barred.is_empty();
        AttackVerdict { complete, addable, issue: found, cost: AttackCost::total(costs, attacks), free }
    }
}

/// The best a creature can obey from its row of what each attack obeys: the most any one attack among `targets` obeys (and
/// not attacking obeys none), with that attack's index; with `free_only`, among attacks that cost nothing (CR 508.1d).
fn best_of(row: &[u32], attacker: ObjectId, targets: &[Target], costs: &[AttackCost], free_only: bool) -> (u32, Option<usize>) {
    let mut best = (0, None);
    for (t, &g) in row.iter().enumerate() {
        if g > best.0 && (!free_only || AttackCost::free(costs, attacker, targets[t])) {
            best = (g, Some(t));
        }
    }
    best
}

/// Creatures placed among the targets of a declaration of attackers, some of which are capped (CR 508.1c): each creature at
/// a capped target, or else at its best attack where nothing caps it (attacking nothing where no such attack obeys
/// anything). A creature's requirements count only its own attack, so what the creatures obey is the sum of what each
/// does, and the caps are room they share: the most they obey is a maximum-weight assignment of creatures to targets with
/// the caps as capacities, which `maximize` finds exactly, in time polynomial in the creatures and targets.
struct Assignment {
    /// Each creature's best attack where nothing caps it: what it obeys, and the target's index.
    open: Vec<(u32, Option<usize>)>,
    /// What each creature obeys attacking each capped target; `None` where it may not (no room, an attack a restriction
    /// forbids, or a cost where only attacks that cost nothing count).
    at_cap: Vec<Vec<Option<u32>>>,
    /// Each capped target's index among the targets, and the room left there.
    capped: Vec<(usize, u32)>,
    /// The capped target each creature attacks, by index into `capped`.
    at: Vec<Option<usize>>,
}

impl Assignment {
    /// `creatures` (each with its row of what each attack obeys) placed beside `fixed`, nobody yet at a capped target; `None`
    /// when `fixed` itself breaks a cap (CR 508.1c). With `free_only`, only attacks that cost nothing count (CR 508.1d).
    fn new(
        rules: &AttackRules,
        rows: &[&[u32]],
        creatures: &[ObjectId],
        targets: &[Target],
        costs: &[AttackCost],
        free_only: bool,
        fixed: &[(ObjectId, Target)],
    ) -> Option<Self> {
        let mut capped: Vec<(usize, u32)> = Vec::new();
        for &(t, most) in &rules.limits {
            let used = count_u32(fixed.iter().filter(|x| x.1 == t).count());
            if used > most {
                return None;
            }
            if let Some(i) = targets.iter().position(|&x| x == t) {
                capped.push((i, most - used));
            }
        }
        let allowed = |a: ObjectId, t: usize| !rules.bars(a, targets[t]) && (!free_only || AttackCost::free(costs, a, targets[t]));
        let open = creatures
            .iter()
            .zip(rows)
            .map(|(&a, row)| {
                let mut best = (0, None);
                for (t, &g) in row.iter().enumerate() {
                    if g > best.0 && !capped.iter().any(|c| c.0 == t) && allowed(a, t) {
                        best = (g, Some(t));
                    }
                }
                best
            })
            .collect();
        let at_cap = creatures.iter().zip(rows).map(|(&a, row)| capped.iter().map(|&(t, room)| (room > 0 && allowed(a, t)).then_some(row[t])).collect()).collect();
        Some(Assignment { open, at_cap, capped, at: vec![None; creatures.len()] })
    }

    /// What creature `i` obeys where it is placed.
    fn value(&self, i: usize) -> u32 {
        match self.at[i] {
            Some(j) => self.at_cap[i][j].unwrap_or(0),
            None => self.open[i].0,
        }
    }

    /// What the creatures obey, placed as they are.
    fn total(&self) -> u32 {
        (0..self.at.len()).map(|i| self.value(i)).sum()
    }

    /// How many creatures attack each capped target.
    fn loads(&self) -> Vec<u32> {
        let mut loads = vec![0u32; self.capped.len()];
        for j in self.at.iter().flatten() {
            loads[*j] += 1;
        }
        loads
    }

    /// The declaration the placing is, in the order of `creatures`.
    fn declaration(&self, creatures: &[ObjectId], targets: &[Target]) -> Vec<(ObjectId, Target)> {
        let mut declaration = Vec::new();
        for (i, &a) in creatures.iter().enumerate() {
            if let Some(j) = self.at[i] {
                declaration.push((a, targets[self.capped[j].0]));
            } else if let (_, Some(t)) = self.open[i] {
                declaration.push((a, targets[t]));
            }
        }
        declaration
    }

    /// What creature `i` gains attacking capped target `j` over its best attack where nothing caps it, where it gains.
    fn more(&self, i: usize, j: usize) -> u32 {
        match self.at_cap[i][j] {
            Some(g) if g > self.open[i].0 => g - self.open[i].0,
            _ => 0,
        }
    }

    /// Places the creatures to obey the most: from nobody at a capped target, by successive augmenting paths - each the one
    /// that adds the most (a creature moved onto a capped target, perhaps moving the one there on to another), until none
    /// adds anything. Each step keeps the placing the best of its size and what a step adds only falls, so it ends at the
    /// best there is (an optimal flow by successive longest paths).
    fn maximize(&mut self) {
        let (n, k) = (self.at.len(), self.capped.len());
        let mut load = self.loads();
        // Each augmenting path takes up one place of room, and there are at most `n` creatures to place.
        for _ in 0..n.min(self.capped.iter().map(|c| c.1 as usize).sum()) {
            // The most a path adds on reaching each creature (0..n) and each capped target (n..n + k): it starts at a
            // creature attacking no capped target, moves it onto a capped target, which may move the creature attacking
            // there onto another, and so on. A path visits each capped target at most once, so k + 1 rounds settle it.
            let mut reach: Vec<Option<i64>> = (0..n + k).map(|v| (v < n && self.at[v].is_none()).then_some(0)).collect();
            let mut came = vec![usize::MAX; k];
            for _ in 0..=k + 1 {
                let mut changed = false;
                for i in 0..n {
                    let Some(d) = reach[i] else { continue };
                    for j in 0..k {
                        let more = self.more(i, j);
                        if more > 0 && self.at[i] != Some(j) {
                            let v = d + i64::from(more);
                            if reach[n + j].is_none_or(|w| v > w) {
                                reach[n + j] = Some(v);
                                came[j] = i;
                                changed = true;
                            }
                        }
                    }
                }
                for i in 0..n {
                    let Some(j) = self.at[i] else { continue };
                    let Some(d) = reach[n + j] else { continue };
                    let v = d - i64::from(self.more(i, j));
                    if reach[i].is_none_or(|w| v > w) {
                        reach[i] = Some(v);
                        changed = true;
                    }
                }
                if !changed {
                    break;
                }
            }
            // The path that adds the most, ending where there is room left; none that adds anything, and this is the most.
            let Some(end) = (0..k).filter(|&j| load[j] < self.capped[j].1 && reach[n + j].is_some_and(|v| v > 0)).max_by_key(|&j| reach[n + j]) else {
                break;
            };
            load[end] += 1;
            let mut j = end;
            for _ in 0..=k {
                let i = came[j];
                let Some(was) = self.at.get_mut(i) else { break };
                match was.replace(j) {
                    Some(prev) => j = prev,
                    None => break,
                }
            }
        }
    }

    /// Fills a place freed at capped target `j` (creature `gone` taken out) the way that obeys the most, and says what that
    /// adds: a creature moves into it from attacking where nothing caps it, or from another capped target, whose place is
    /// filled the same way, and so on; or the place stays empty, adding nothing. From the best placing for the room there
    /// was, this is the best for one place more (an optimal flow, and the best cycle through one unit of capacity more).
    fn fill(&mut self, j: usize, gone: usize) -> i64 {
        #[derive(Clone, Copy)]
        enum From {
            Nobody,
            Open(usize),
            Capped(usize, usize),
        }
        let k = self.capped.len();
        let mut best: Vec<(i64, From)> = vec![(0, From::Nobody); k];
        for i in (0..self.at.len()).filter(|&i| i != gone && self.at[i].is_none()) {
            for (u, slot) in best.iter_mut().enumerate() {
                if let Some(g) = self.at_cap[i][u] {
                    let v = i64::from(g) - i64::from(self.open[i].0);
                    if v > slot.0 {
                        *slot = (v, From::Open(i));
                    }
                }
            }
        }
        // A path visits each capped target at most once, so k + 1 rounds settle it.
        for _ in 0..=k {
            let mut changed = false;
            for i in (0..self.at.len()).filter(|&i| i != gone) {
                let Some(v) = self.at[i] else { continue };
                let here = i64::from(self.at_cap[i][v].unwrap_or(0));
                for u in (0..k).filter(|&u| u != v) {
                    if let Some(g) = self.at_cap[i][u] {
                        let w = i64::from(g) - here + best[v].0;
                        if w > best[u].0 {
                            best[u] = (w, From::Capped(i, v));
                            changed = true;
                        }
                    }
                }
            }
            if !changed {
                break;
            }
        }
        let gained = best[j].0;
        let mut u = j;
        for _ in 0..=k {
            match best[u].1 {
                From::Nobody => break,
                From::Open(i) => {
                    self.at[i] = Some(u);
                    break;
                }
                From::Capped(i, v) => {
                    self.at[i] = Some(u);
                    u = v;
                }
            }
        }
        gained
    }

    /// What the creatures other than `gone` obey more (never more than nothing, from the best placing) once one place at
    /// each capped target is made free the best way, the placing left as it is: a creature there moves to attacking where
    /// nothing caps it, or to a capped target with room, or to a full one that makes room the same way. From the best
    /// placing, this is the best with one place less there (an optimal flow, and the best cycle through one unit of
    /// capacity less). `None` where nobody attacks it.
    fn shed(&self, gone: usize) -> Vec<Option<i64>> {
        fn raise(slot: &mut Option<i64>, v: i64) -> bool {
            let higher = slot.is_none_or(|x| v > x);
            if higher {
                *slot = Some(v);
            }
            higher
        }
        let k = self.capped.len();
        let loads = self.loads();
        let spare: Vec<bool> = (0..k).map(|j| loads[j] < self.capped[j].1).collect();
        let mut best: Vec<Option<i64>> = vec![None; k];
        let placed: Vec<(usize, usize)> = (0..self.at.len()).filter(|&i| i != gone).filter_map(|i| self.at[i].map(|u| (i, u))).collect();
        for &(i, u) in &placed {
            let here = i64::from(self.at_cap[i][u].unwrap_or(0));
            raise(&mut best[u], i64::from(self.open[i].0) - here);
            for w in (0..k).filter(|&w| w != u && spare[w]) {
                if let Some(g) = self.at_cap[i][w] {
                    raise(&mut best[u], i64::from(g) - here);
                }
            }
        }
        // A path visits each capped target at most once, so k + 1 rounds settle it.
        for _ in 0..=k {
            let mut changed = false;
            for &(i, u) in &placed {
                let here = i64::from(self.at_cap[i][u].unwrap_or(0));
                for w in (0..k).filter(|&w| w != u && !spare[w]) {
                    if let (Some(g), Some(o)) = (self.at_cap[i][w], best[w]) {
                        changed |= raise(&mut best[u], i64::from(g) - here + o);
                    }
                }
            }
            if !changed {
                break;
            }
        }
        best
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const U: ObjectId = ObjectId(5);
    const V: ObjectId = ObjectId(6);
    const P2: PlayerId = PlayerId(1);
    const P3: PlayerId = PlayerId(2);
    /// A planeswalker of P2 that one creature at most may attack (Tomik, Orzhov Lawmage).
    const W: Target = Target::Object(ObjectId(7));

    fn capped(at: Target) -> AttackRules {
        AttackRules { limits: vec![(at, 1)], ..Default::default() }
    }

    /// Goaded by P2 and told to attack P2 this combat (three players, Ursine Monstrosity): attacking P2 obeys two of three
    /// (attack, and attack P2), attacking P3 two (attack, and attack a player other than P2); both are legal, and not
    /// attacking is refused.
    #[test]
    fn a_goaded_creature_told_to_attack_its_goader_may_attack_either_player() {
        let targets = [Target::Player(P2), Target::Player(P3)];
        let mut r = AttackRules {
            required: vec![
                RequiredAttack { attacker: U, aim: AttackAim::Anything },
                RequiredAttack { attacker: U, aim: AttackAim::PlayerOtherThan(P2) },
                RequiredAttack { attacker: U, aim: AttackAim::At(Target::Player(P2)) },
            ],
            ..Default::default()
        };
        (r.most, r.suggested) = r.solve(&[U], &targets, &[]);
        assert_eq!(r.most, 2);
        assert!(r.judge(&[U], &targets, &[], &[(U, Target::Player(P2))], false).complete);
        assert!(r.judge(&[U], &targets, &[], &[(U, Target::Player(P3))], false).complete);
        let none = r.judge(&[U], &targets, &[], &[], true);
        assert!(!none.complete && none.addable.len() == 2);
        assert_eq!(none.issue.map(|i| i.rule), Some("508.1d"));
    }

    /// "Each creature attacks this turn if able" (Goblin Diplomats) on the only creature able to attack, which can't attack
    /// alone (Pet Sehlat, CR 508.1c): it can obey nothing, so no attack is legal and attacking alone is not (CR 508.1d).
    #[test]
    fn a_lone_creature_that_cannot_attack_alone_is_required_nothing() {
        let targets = [Target::Player(P2)];
        let mut r = AttackRules { required: vec![RequiredAttack { attacker: U, aim: AttackAim::Anything }], alone_barred: vec![U], ..Default::default() };
        (r.most, r.suggested) = r.solve(&[U], &targets, &[]);
        assert_eq!((r.most, r.suggested.clone()), (0, Vec::new()));
        let none = r.judge(&[U], &targets, &[], &[], true);
        assert!(none.complete && none.addable.is_empty() && !none.free);
        let alone = r.judge(&[U], &targets, &[], &[(U, Target::Player(P2))], true);
        assert_eq!(alone.issue.map(|i| (i.words, i.rule)), Some(("can't attack alone".to_string(), "508.1c")));
    }

    /// A creature that can't attack alone may be added to nothing only where another creature can attack beside it: alone
    /// against a capped planeswalker and a player, never; beside a creature that may attack only the planeswalker, which
    /// takes one, only at the player. Against every declaration there is.
    #[test]
    fn a_creature_that_cannot_attack_alone_is_addable_only_beside_another() {
        let b = Target::Player(P2);
        let menu: [&[AttackAim]; 3] = [&[], &[AttackAim::Anything], &[AttackAim::At(W)]];
        let caps: [&[(Target, u32)]; 3] = [&[], &[(W, 1)], &[(W, 1), (b, 1)]];
        check_every_declaration(&[U], &[b, W], &menu, &caps, &[&[]], &[&[U]], &[&[]], &[Vec::new()]);
        let partial = vec![Vec::new(), vec![(U, b)], vec![(U, W)], vec![(V, W)]];
        check_every_declaration(&[U, V], &[b, W], &menu, &caps, &[&[], &[(V, b)]], &[&[U], &[U, V]], &[&[]], &partial);
        let r = AttackRules { limits: vec![(W, 1)], barred: vec![(V, b)], alone_barred: vec![U], ..Default::default() };
        assert_eq!(r.judge(&[U, V], &[b, W], &[], &[], true).addable, vec![(U, b), (V, W)]);
    }

    /// Two creatures that can't attack alone may attack together: either may be added first, and the other then mends it.
    #[test]
    fn two_creatures_that_cannot_attack_alone_may_attack_together() {
        let b = Target::Player(P2);
        let r = AttackRules { alone_barred: vec![U, V], ..Default::default() };
        let none = r.judge(&[U, V], &[b], &[], &[], true);
        assert_eq!(none.addable, vec![(U, b), (V, b)]);
        assert!(!none.free);
        let one = r.judge(&[U, V], &[b], &[], &[(U, b)], true);
        assert!(!one.complete && one.addable == vec![(V, b)]);
        assert!(r.judge(&[U, V], &[b], &[], &[(U, b), (V, b)], false).complete);
    }

    /// A cost to attack is never paid to obey a requirement (CR 508.1d): with every attack costing something, nothing is
    /// required, and attacking and paying is still legal.
    #[test]
    fn a_requirement_is_never_obeyed_by_paying() {
        let targets = [Target::Player(P2)];
        let costs = [AttackCost { attacker: U, target: Target::Player(P2), cost: ManaCost { generic: 2, ..ManaCost::ZERO }, source: ObjectId(9) }];
        let mut r = AttackRules { required: vec![RequiredAttack { attacker: U, aim: AttackAim::Anything }], ..Default::default() };
        (r.most, r.suggested) = r.solve(&[U], &targets, &costs);
        assert_eq!(r.most, 0);
        assert!(r.judge(&[U], &targets, &costs, &[], false).complete);
        let paid = r.judge(&[U], &targets, &costs, &[(U, Target::Player(P2))], false);
        assert!(paid.complete && paid.cost.generic == 2);
    }

    /// CR 508.1c: a second creature declared attacking a planeswalker no more than one may attack makes the declaration
    /// illegal, and the issue is the caption's.
    #[test]
    fn a_second_attacker_on_a_capped_target_is_illegal() {
        let targets = [Target::Player(P2), W];
        let verdict = capped(W).judge(&[U, V], &targets, &[], &[(U, W), (V, W)], false);
        assert!(!verdict.complete);
        let issue = verdict.issue.expect("an illegal declaration says why");
        assert_eq!((issue.creature, issue.words.as_str(), issue.other, issue.rule), (V, "can't attack", Some(W), "508.1c"));
    }

    /// One creature attacking the capped planeswalker, or another target, is legal.
    #[test]
    fn a_capped_target_allows_one_attacker_and_no_cap_on_another() {
        let targets = [Target::Player(P2), W];
        assert!(capped(W).judge(&[U, V], &targets, &[], &[(U, W)], false).complete);
        assert!(capped(W).judge(&[U, V], &targets, &[], &[(U, W), (V, Target::Player(P2))], false).complete);
        assert!(capped(W).judge(&[U, V], &targets, &[], &[(U, Target::Player(P2)), (V, Target::Player(P2))], false).complete);
    }

    /// Two creatures may attack one target where nothing caps it.
    #[test]
    fn without_a_limit_two_may_attack_one_target() {
        let targets = [Target::Player(P2), W];
        assert!(AttackRules::default().judge(&[U, V], &targets, &[], &[(U, W), (V, W)], false).complete);
    }

    /// Nothing is addable onto a target already at its cap, and what is addable elsewhere still is.
    #[test]
    fn addable_never_adds_a_second_attacker_to_a_capped_target() {
        let targets = [Target::Player(P2), W];
        let verdict = capped(W).judge(&[U, V], &targets, &[], &[(U, W)], true);
        assert!(verdict.complete);
        assert!(verdict.addable.contains(&(V, Target::Player(P2))));
        assert!(!verdict.addable.contains(&(V, W)));
        // Two attacks it offers may still be one too many here, so a client has to ask.
        assert!(!verdict.free);
    }

    /// The most is over declarations, not creatures: with a cap of one on Player B, a creature that must attack Player B
    /// takes the one place there and the creature that may attack anything goes to the planeswalker (CR 508.1d). Filling
    /// Player B with the wrong creature leaves the declaration one requirement short, and is refused.
    #[test]
    fn a_cap_does_not_hide_a_requirement_another_creature_can_obey() {
        let b = Target::Player(P2);
        let targets = [b, W];
        let mut r = capped(b);
        r.required = vec![
            RequiredAttack { attacker: U, aim: AttackAim::Anything },
            RequiredAttack { attacker: V, aim: AttackAim::At(b) },
        ];
        (r.most, r.suggested) = r.solve(&[U, V], &targets, &[]);
        assert_eq!(r.most, 2);
        assert_eq!(r.suggested, vec![(U, W), (V, b)]);
        assert!(r.judge(&[U, V], &targets, &[], &r.suggested, false).complete);
        let wrong = r.judge(&[U, V], &targets, &[], &[(U, b)], false);
        assert!(!wrong.complete);
        assert_eq!(wrong.issue.as_ref().map(|i| (i.creature, i.words.as_str(), i.other, i.rule)), Some((V, "must attack", Some(b), "508.1d")));
    }

    /// Where no cap binds, the most is each creature's best attack added up: two that may attack anything each attack.
    #[test]
    fn a_cap_that_does_not_bind_leaves_the_most_added_up() {
        let b = Target::Player(P2);
        let targets = [b, W];
        let mut r = capped(b);
        r.required = vec![RequiredAttack { attacker: U, aim: AttackAim::Anything }, RequiredAttack { attacker: V, aim: AttackAim::Anything }];
        (r.most, r.suggested) = r.solve(&[U, V], &targets, &[]);
        assert_eq!(r.most, 2);
        assert_eq!(r.suggested.len(), 2);
        assert!(r.suggested.iter().filter(|x| x.1 == b).count() <= 1);
        assert!(r.judge(&[U, V], &targets, &[], &r.suggested, false).complete);
        assert!(r.judge(&[U, V], &targets, &[], &[(U, b), (V, W)], false).complete);
        assert!(!r.judge(&[U, V], &targets, &[], &[(U, b), (V, b)], false).complete);
    }

    /// Two creatures competing for one place at a capped target cannot both take it (CR 508.1c): an attack is addable only
    /// where some declaration extending this one with it still obeys the most (CR 508.1d), so a place at a capped target
    /// that a creature which must attack there needs is not offered to a creature that obeys nothing.
    #[test]
    fn addable_does_not_count_two_creatures_into_one_capped_place() {
        const Y: ObjectId = ObjectId(8);
        const Z: ObjectId = ObjectId(9);
        let b = Target::Player(P2);
        let targets = [b, W];
        let mut r = AttackRules { limits: vec![(b, 2)], ..Default::default() };
        r.required = vec![
            RequiredAttack { attacker: U, aim: AttackAim::Anything },
            RequiredAttack { attacker: V, aim: AttackAim::Anything },
            RequiredAttack { attacker: V, aim: AttackAim::At(b) },
            RequiredAttack { attacker: Y, aim: AttackAim::Anything },
            RequiredAttack { attacker: Y, aim: AttackAim::At(b) },
        ];
        let eligible = [U, V, Y, Z];
        (r.most, r.suggested) = r.solve(&eligible, &targets, &[]);
        assert_eq!(r.most, 5);
        // U holds a place at Player B that only one of V and Y can make up: every declaration extending this one obeys 4.
        let verdict = r.judge(&eligible, &targets, &[], &[(U, b)], true);
        assert!(!verdict.complete);
        assert!(verdict.addable.is_empty());
    }

    /// The suggestion fills a capped target no further than the cap, even where a requirement would send another there.
    #[test]
    fn a_suggestion_never_exceeds_a_limit() {
        let targets = [Target::Player(P2), W];
        let mut r = capped(Target::Player(P2));
        r.required = vec![
            RequiredAttack { attacker: U, aim: AttackAim::Anything },
            RequiredAttack { attacker: U, aim: AttackAim::At(Target::Player(P2)) },
            RequiredAttack { attacker: V, aim: AttackAim::Anything },
            RequiredAttack { attacker: V, aim: AttackAim::At(Target::Player(P2)) },
        ];
        (r.most, r.suggested) = r.solve(&[U, V], &targets, &[]);
        assert_eq!(r.suggested.iter().filter(|x| x.1 == Target::Player(P2)).count(), 1);
        assert_eq!(r.most, 3);
        assert!(r.judge(&[U, V], &targets, &[], &r.suggested, false).complete);
    }

    /// Every declaration of `creatures` among `targets`: each creature attacking one of them or nothing.
    fn every_declaration(creatures: &[ObjectId], targets: &[Target]) -> Vec<Vec<(ObjectId, Target)>> {
        let mut all = vec![Vec::new()];
        for &a in creatures {
            all = all
                .into_iter()
                .flat_map(|d: Vec<(ObjectId, Target)>| {
                    let mut next = vec![d.clone()];
                    for &t in targets {
                        let mut with = d.clone();
                        with.push((a, t));
                        next.push(with);
                    }
                    next
                })
                .collect();
        }
        all
    }

    /// Checks the most, whether each declaration may be made and what may be added to each of `partial`, against every
    /// declaration there is (CR 508.1c-d), for `creatures` under every mix of `menu`'s requirements, each of `caps`, each
    /// of `bars` (the attacks a restriction forbids), each of `alone` (the creatures forbidden to attack alone) and each of
    /// `costs`. A declaration within the caps that makes no forbidden attack, and is not one of those creatures attacking
    /// alone, is legal where it obeys as many as any such declaration that pays nothing; an attack is addable where some
    /// legal declaration extends the declaration with it.
    #[allow(clippy::too_many_arguments)]
    fn check_every_declaration(
        creatures: &[ObjectId],
        targets: &[Target],
        menu: &[&[AttackAim]],
        caps: &[&[(Target, u32)]],
        bars: &[&[(ObjectId, Target)]],
        alone: &[&[ObjectId]],
        costs: &[&[AttackCost]],
        partial: &[Vec<(ObjectId, Target)>],
    ) {
        let declarations = every_declaration(creatures, targets);
        for &costs in costs {
            for (&limits, &barred, &alone) in caps.iter().flat_map(|c| bars.iter().flat_map(move |b| alone.iter().map(move |l| (c, b, l)))) {
                for x in 0..menu.len().pow(count_u32(creatures.len())) {
                    let picks: Vec<usize> = (0..creatures.len()).map(|i| x / menu.len().pow(count_u32(i)) % menu.len()).collect();
                    let required = creatures
                        .iter()
                        .zip(&picks)
                        .flat_map(|(&attacker, &p)| menu[p].iter().map(move |&aim| RequiredAttack { attacker, aim }))
                        .collect();
                    let mut r = AttackRules { required, limits: limits.to_vec(), barred: barred.to_vec(), alone_barred: alone.to_vec(), ..Default::default() };
                    let fits = |d: &[(ObjectId, Target)]| {
                        limits.iter().all(|&(t, n)| count_u32(d.iter().filter(|x| x.1 == t).count()) <= n) && !d.iter().any(|x| barred.contains(x))
                    };
                    let within = |d: &[(ObjectId, Target)]| fits(d) && !matches!(d, [(a, _)] if alone.contains(a));
                    let free = |d: &[(ObjectId, Target)]| d.iter().all(|&(a, t)| AttackCost::free(costs, a, t));
                    let most = declarations.iter().filter(|d| within(d) && free(d)).map(|d| r.obeyed(d)).max().unwrap_or(0);
                    (r.most, r.suggested) = r.solve(creatures, targets, costs);
                    assert_eq!(r.most, most, "{limits:?} {barred:?} {alone:?} {picks:?}");
                    assert!(within(&r.suggested) && free(&r.suggested) && r.obeyed(&r.suggested) == most, "{limits:?} {barred:?} {alone:?} {picks:?}");
                    let mut legal = Vec::new();
                    for d in &declarations {
                        let ok = within(d) && r.obeyed(d) >= most;
                        let verdict = r.judge(creatures, targets, costs, d, false);
                        assert_eq!(verdict.complete, ok, "{limits:?} {barred:?} {alone:?} {picks:?} {d:?}");
                        assert!(!verdict.free || alone.is_empty());
                        if ok {
                            legal.push(d);
                        }
                    }
                    // A creature forbidden to attack alone, declared alone, is a partial declaration another attacker mends.
                    for p in partial.iter().filter(|p| fits(p)) {
                        let mut want: Vec<(ObjectId, Target)> = Vec::new();
                        for &a in creatures.iter().filter(|&&a| !p.iter().any(|x| x.0 == a)) {
                            for &t in targets {
                                if legal.iter().any(|d| d.contains(&(a, t)) && p.iter().all(|x| d.contains(x))) {
                                    want.push((a, t));
                                }
                            }
                        }
                        assert_eq!(r.judge(creatures, targets, costs, p, true).addable, want, "{limits:?} {barred:?} {alone:?} {picks:?} {p:?}");
                    }
                }
            }
        }
    }

    /// Three creatures under every mix of requirements a goad, "attacks each combat if able" and "attacks that player or
    /// planeswalker if able" make, three targets under caps of none, zero, one and two, with and without a cost to one
    /// attack, and with no attack forbidden or with a restriction forbidding U to attack Player B and V to attack the
    /// planeswalker ("creatures with flying can't attack you", Storm, Windrider): the most, every declaration and what may
    /// be added to an empty or one-attack declaration match every declaration there is.
    #[test]
    fn the_most_and_addable_match_every_declaration() {
        const Y: ObjectId = ObjectId(8);
        let creatures = [U, V, Y];
        let targets = [Target::Player(P2), Target::Player(P3), W];
        let menu: [&[AttackAim]; 8] = [
            &[],
            &[AttackAim::Anything],
            &[AttackAim::At(Target::Player(P2))],
            &[AttackAim::At(W)],
            &[AttackAim::Anything, AttackAim::At(W)],
            &[AttackAim::Anything, AttackAim::PlayerOtherThan(P2)],
            &[AttackAim::Anything, AttackAim::PlayerOtherThan(P2), AttackAim::At(Target::Player(P2))],
            &[AttackAim::Anything, AttackAim::Anything],
        ];
        let caps: [&[(Target, u32)]; 7] = [
            &[],
            &[(W, 1)],
            &[(W, 0)],
            &[(Target::Player(P2), 1)],
            &[(Target::Player(P2), 1), (W, 1)],
            &[(Target::Player(P2), 2), (Target::Player(P3), 1), (W, 1)],
            &[(Target::Player(P2), 1), (Target::Player(P3), 0), (W, 2)],
        ];
        let cost = [AttackCost { attacker: U, target: Target::Player(P3), cost: ManaCost { generic: 2, ..ManaCost::ZERO }, source: ObjectId(20) }];
        let mut partial = vec![Vec::new()];
        partial.extend(creatures.iter().flat_map(|&a| targets.iter().map(move |&t| vec![(a, t)])));
        let bars: [&[(ObjectId, Target)]; 2] = [&[], &[(U, Target::Player(P2)), (V, W)]];
        check_every_declaration(&creatures, &targets, &menu, &caps, &bars, &[&[], &[U], &[U, V]], &[&[], &cost], &partial);
    }

    /// Four creatures against three targets all capped, where taking a creature out of the best declaration and putting it
    /// elsewhere moves others along a chain of capped targets: the most, every declaration and what may be added to an
    /// empty, one-attack or two-attack declaration match every declaration there is.
    #[test]
    fn addable_matches_every_declaration_along_chains_of_caps() {
        const Y: ObjectId = ObjectId(8);
        const Z: ObjectId = ObjectId(9);
        let creatures = [U, V, Y, Z];
        let targets = [Target::Player(P2), Target::Player(P3), W];
        let menu: [&[AttackAim]; 5] = [
            &[],
            &[AttackAim::Anything],
            &[AttackAim::Anything, AttackAim::At(W)],
            &[AttackAim::Anything, AttackAim::At(Target::Player(P2))],
            &[AttackAim::At(Target::Player(P3)), AttackAim::At(Target::Player(P3))],
        ];
        let caps: [&[(Target, u32)]; 4] = [
            &[(Target::Player(P2), 1), (Target::Player(P3), 1), (W, 1)],
            &[(Target::Player(P2), 2), (Target::Player(P3), 1), (W, 1)],
            &[(Target::Player(P2), 1), (W, 1)],
            &[(W, 2), (Target::Player(P3), 1)],
        ];
        let mut partial = vec![Vec::new()];
        partial.extend(creatures.iter().flat_map(|&a| targets.iter().map(move |&t| vec![(a, t)])));
        for (i, &a) in creatures.iter().enumerate() {
            for &b in &creatures[i + 1..] {
                for &t in &targets {
                    for &u in &targets {
                        partial.push(vec![(a, t), (b, u)]);
                    }
                }
            }
        }
        let bars: [&[(ObjectId, Target)]; 2] = [&[], &[(V, Target::Player(P2)), (Y, W), (Z, Target::Player(P3))]];
        check_every_declaration(&creatures, &targets, &menu, &caps, &bars, &[&[], &[U, Z]], &[&[]], &partial);
    }

    /// Making room at a full capped target can take a chain of moves: with a cap of one on Player C and on the planeswalker,
    /// U (three requirements to attack the planeswalker, two to attack Player C) there and V (attack, and attack Player C)
    /// at Player C obey 5; Y (two to attack the planeswalker) may still attack it, as Y there, U at Player C and V at Player
    /// B obey 5 too - U moved on, and V out of U's way. Every mix of these requirements matches every declaration there is.
    #[test]
    fn room_at_a_capped_target_is_made_along_a_chain() {
        const Y: ObjectId = ObjectId(8);
        let (b, c) = (Target::Player(P2), Target::Player(P3));
        let creatures = [U, V, Y];
        let targets = [b, c, W];
        let menu: [&[AttackAim]; 4] = [
            &[],
            &[AttackAim::At(W), AttackAim::At(W), AttackAim::At(W), AttackAim::At(c), AttackAim::At(c)],
            &[AttackAim::Anything, AttackAim::At(c)],
            &[AttackAim::At(W), AttackAim::At(W)],
        ];
        let mut r = AttackRules { limits: vec![(c, 1), (W, 1)], ..Default::default() };
        r.required = [(U, 1), (V, 2), (Y, 3)].iter().flat_map(|&(attacker, p)| menu[p].iter().map(move |&aim| RequiredAttack { attacker, aim })).collect();
        (r.most, r.suggested) = r.solve(&creatures, &targets, &[]);
        assert_eq!(r.most, 5);
        assert!(r.judge(&creatures, &targets, &[], &[], true).addable.contains(&(Y, W)));
        check_every_declaration(&creatures, &targets, &menu, &[&[(c, 1), (W, 1)]], &[&[]], &[&[], &[U], &[Y]], &[&[]], &[Vec::new(), vec![(Y, W)], vec![(V, b)]]);
    }

    /// A placing written out: each creature's best open attack (what it obeys), what it obeys at each capped target, where
    /// it is, and the room at each capped target.
    fn placing(open: &[u32], at_cap: &[[Option<u32>; 4]], at: &[Option<usize>], rooms: &[u32]) -> Assignment {
        Assignment {
            open: open.iter().map(|&g| (g, None)).collect(),
            at_cap: at_cap.iter().map(|r| r.to_vec()).collect(),
            capped: rooms.iter().copied().enumerate().collect(),
            at: at.to_vec(),
        }
    }

    /// Making room at a full capped target down a chain of three moves, which the creatures' order makes take a round
    /// each: the creature at the first target moves to the second, the one there to the third, the one there to the
    /// fourth, and the one there goes where nothing caps it for nothing; each move costs 1.
    #[test]
    fn shed_follows_a_chain_a_round_at_a_time() {
        let a = placing(
            &[0, 0, 0, 10],
            &[
                [Some(10), Some(9), None, None],
                [None, Some(10), Some(9), None],
                [None, None, Some(10), Some(9)],
                [None, None, None, Some(10)],
            ],
            &[Some(0), Some(1), Some(2), Some(3)],
            &[1, 1, 1, 1],
        );
        assert_eq!(a.shed(usize::MAX), vec![Some(-3), Some(-2), Some(-1), Some(0)]);
    }

    /// Filling a place freed at the last capped target down a chain of three moves, a round each: the creature at the
    /// third target moves into it, the one at the second into the third, the one at the first into the second, and a
    /// creature attacking where nothing caps it into the first, for 5 in all.
    #[test]
    fn fill_follows_a_chain_a_round_at_a_time() {
        let mut a = placing(
            &[0, 0, 0, 0, 0],
            &[
                [None, None, Some(6), Some(6)],
                [None, Some(6), Some(6), None],
                [Some(6), Some(6), None, None],
                [Some(5), None, None, None],
                [None, None, None, Some(100)],
            ],
            &[Some(2), Some(1), Some(0), None, None],
            &[1, 1, 1, 1],
        );
        assert_eq!(a.fill(3, 4), 5);
        assert_eq!(a.at, vec![Some(3), Some(2), Some(1), Some(0), None]);
    }

    /// The assignment takes time polynomial in the creatures and targets: sixty creatures that each attack if able, against
    /// six capped planeswalkers and a capped player, obey as many as there are places.
    #[test]
    fn many_creatures_against_many_caps_are_solved_at_once() {
        let creatures: Vec<ObjectId> = (100..160).map(ObjectId).collect();
        let mut targets = vec![Target::Player(P2)];
        targets.extend((200..206).map(|i| Target::Object(ObjectId(i))));
        let mut r = AttackRules {
            required: creatures.iter().map(|&attacker| RequiredAttack { attacker, aim: AttackAim::Anything }).collect(),
            limits: targets.iter().map(|&t| (t, 1)).collect(),
            ..Default::default()
        };
        (r.most, r.suggested) = r.solve(&creatures, &targets, &[]);
        assert_eq!(r.most, 7);
        assert!(r.judge(&creatures, &targets, &[], &r.suggested, false).complete);
        let verdict = r.judge(&creatures, &targets, &[], &r.suggested[..3], true);
        assert!(!verdict.complete);
        assert_eq!(verdict.addable.len(), (creatures.len() - 3) * 4);
    }

    /// What may be added to a declaration, which the host works out at every declaration of attackers, takes well under a
    /// second for four hundred creatures that each attack if able against six capped planeswalkers and a player nothing
    /// caps, and as long again with the planeswalkers wanted: one best declaration answers every attack.
    #[test]
    fn addable_for_hundreds_of_creatures_is_quick() {
        let creatures: Vec<ObjectId> = (1000..1400).map(ObjectId).collect();
        let walkers: Vec<Target> = (200..206).map(|i| Target::Object(ObjectId(i))).collect();
        let mut targets = vec![Target::Player(P2)];
        targets.extend(&walkers);
        let mut required: Vec<RequiredAttack> = creatures.iter().map(|&attacker| RequiredAttack { attacker, aim: AttackAim::Anything }).collect();
        let mut r = AttackRules { required: required.clone(), limits: walkers.iter().map(|&t| (t, 1)).collect(), ..Default::default() };
        // Every other creature also told to attack a planeswalker, one each in turn: six of them get there.
        required.extend(creatures.iter().step_by(2).zip(walkers.iter().cycle()).map(|(&attacker, &t)| RequiredAttack { attacker, aim: AttackAim::At(t) }));
        let mut wanted = AttackRules { required, ..r.clone() };
        let started = std::time::Instant::now();
        (r.most, r.suggested) = r.solve(&creatures, &targets, &[]);
        (wanted.most, wanted.suggested) = wanted.solve(&creatures, &targets, &[]);
        assert_eq!((r.most, wanted.most), (400, 406));
        let none = r.judge(&creatures, &targets, &[], &[], true);
        let half = r.judge(&creatures, &targets, &[], &r.suggested[..200], true);
        let some = wanted.judge(&creatures, &targets, &[], &wanted.suggested[..200], true);
        let elapsed = started.elapsed();
        assert_eq!((none.addable.len(), half.addable.len()), (400 * 7, 200 * 7));
        assert!(!some.addable.is_empty());
        assert!(elapsed < std::time::Duration::from_secs(3), "took {elapsed:?}");
    }
}
