//! Judging a declaration of blockers (CR 509.1a-d): one validator for every restriction, requirement and cost a declaration
//! answers to, from the rules the question holds (`BlockRequirements`). `BlockRequirements::judge` says whether a declaration
//! may be made as it is (`BlockVerdict::complete`), why not (`BlockVerdict::issue`), what it costs (`BlockVerdict::cost`), and
//! which blocks may be added to it on the way to one that may (`BlockVerdict::addable`). The core checks every answer with
//! it, the scenario harness reads it, and the host gives its verdict to a client as its player declares (`CheckBlocks`), so no
//! client works the rules out and a new restriction or requirement changes the engine alone.
//!
//! **"Addable" means that some legal declaration extends the declaration with the block** - not that the declaration with
//! it is legal now: one blocker of a menace creature is no declaration, two are, and a declaration short of a requirement
//! (CR 509.1c) is completed only by adding the block it requires. A block that no legal declaration has alongside the blocks
//! declared is not addable; taking a block back out is always allowed.
//!
//! `solve` is the one search: the most requirements a declaration can obey without paying a cost (asked once as the question
//! is made), and whether a declaration that obeys that many extends given blocks (each addable block).

use crate::{BlockRequirements, ManaCost, ObjectId, count_u32};

/// What the rules make of a declaration of blockers.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct BlockVerdict {
    /// It may be declared as it is: every block allowed, each blocker blocking as many creatures as it may, each attacker
    /// blocked by as many creatures as it may be, and as many requirements obeyed as any declaration can (CR 509.1a-c). Its
    /// cost, if any, is paid once it is declared (CR 509.1d), and declaring it is taken back if it isn't (CR 733).
    pub complete: bool,
    /// The blocks that some legal declaration extending it has, among the blocks not declared (empty unless asked for).
    pub addable: Vec<(ObjectId, ObjectId)>,
    /// Why it may not be declared as it is; none when it may.
    pub issue: Option<BlockIssue>,
    /// What declaring it costs (CR 509.1d), zero for a declaration that costs nothing.
    pub cost: ManaCost,
}

/// Why a declaration of blockers may not be made as it is, as a caption reads it: `creature`, then `words`, then `other`
/// where there is one ("Grizzly Bears" "must block" "Goblin Guide"; "Boggart Brute" "can't be blocked except by two or more
/// creatures"), with the rule it breaks.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct BlockIssue {
    pub creature: ObjectId,
    pub words: String,
    pub other: Option<ObjectId>,
    /// The rule, as the Comprehensive Rules number it ("509.1c").
    pub rule: &'static str,
}

fn creatures(n: u32) -> String {
    const WORDS: [&str; 10] = ["no", "one", "two", "three", "four", "five", "six", "seven", "eight", "nine"];
    let number = WORDS.get(n as usize).map_or_else(|| n.to_string(), |w| (*w).to_string());
    if n == 1 { format!("{number} creature") } else { format!("{number} creatures") }
}

#[cfg(test)]
thread_local! { static MULTI_SEARCH_NODES: std::cell::Cell<usize> = const { std::cell::Cell::new(0) }; }

impl BlockRequirements {
    /// What the rules make of `blocks`, a declaration among the `allowed` blocks (CR 509.1a-d); with `addable`, which blocks
    /// may be added on the way to a legal one too. Without it the verdict is a walk over the blocks, cheap enough to check
    /// every answer with.
    pub fn judge(&self, allowed: &[(ObjectId, ObjectId)], blocks: &[(ObjectId, ObjectId)], addable: bool) -> BlockVerdict {
        let (issue, extendable) = match self.issue(allowed, blocks) {
            Some((issue, extendable)) => (Some(issue), extendable),
            None => (None, true),
        };
        let complete = issue.is_none();
        let addable = if addable && extendable {
            allowed
                .iter()
                .copied()
                .filter(|&(b, a)| {
                    !blocks.contains(&(b, a))
                        && count_u32(blocks.iter().filter(|x| x.0 == b).count()) < self.capacity_of(b)
                        && count_u32(blocks.iter().filter(|x| x.1 == a).count()) < self.limits_of(a).1
                        && {
                            let mut more = blocks.to_vec();
                            more.push((b, a));
                            self.issue(allowed, &more).is_none() || self.solve(allowed, &more, Some(self.most), false).is_some()
                        }
                })
                .collect()
        } else {
            Vec::new()
        };
        BlockVerdict { complete, addable, issue, cost: self.total_cost(blocks) }
    }

    /// The first thing that keeps `blocks` from being declared as it is, if anything does, and whether blocks added to it could
    /// mend it: not past a block that is not allowed, a creature blocking twice or an attacker over its most.
    fn issue(&self, allowed: &[(ObjectId, ObjectId)], blocks: &[(ObjectId, ObjectId)]) -> Option<(BlockIssue, bool)> {
        let issue = |creature, words: String, other, rule, mendable| Some((BlockIssue { creature, words, other, rule }, mendable));
        for (i, &(b, a)) in blocks.iter().enumerate() {
            if blocks[..i].contains(&(b, a)) {
                return issue(b, "can't block the same creature twice".into(), Some(a), "509.1a", false);
            }
            let capacity = self.capacity_of(b);
            if count_u32(blocks[..i].iter().filter(|x| x.0 == b).count()) >= capacity {
                let words = if capacity == 1 { "can block only one creature".into() }
                    else { format!("can't block more than {}", creatures(capacity)) };
                return issue(b, words, None, "509.1a", false);
            }
            if !allowed.contains(&(b, a)) {
                return issue(b, "can't block".into(), Some(a), "509.1b", false);
            }
        }
        for (i, &(_, a)) in blocks.iter().enumerate() {
            if blocks[..i].iter().any(|x| x.1 == a) {
                continue;
            }
            let n = count_u32(blocks.iter().filter(|x| x.1 == a).count());
            let (least, most) = self.limits_of(a);
            if n > most {
                return issue(a, format!("can't be blocked by more than {}", creatures(most)), None, "509.1b", false);
            }
            if n < least {
                return issue(a, format!("can't be blocked except by {} or more", creatures(least)), None, "509.1b", true);
            }
        }
        if self.obeyed(blocks) < self.most {
            // A requirement left unobeyed by a creature that obeys none, so the caption names a creature that can still obey
            // it; any unobeyed one where every such creature obeys another.
            let unobeyed = |r: &&crate::RequiredBlock| !blocks.iter().any(|&(b, x)| self.obeys(r, b, x));
            let obeys_any = |c: ObjectId| blocks.iter().any(|&(b, x)| b == c && self.required.iter().any(|r| self.obeys(r, b, x)));
            let r = self
                .required
                .iter()
                .filter(unobeyed)
                .find(|r| r.blocker.is_none_or(|b| !obeys_any(b)))
                .or_else(|| self.required.iter().find(unobeyed))?;
            return match (r.blocker, r.attacker) {
                (Some(b), other) => issue(b, "must block".into(), other, "509.1c", true),
                (None, Some(a)) => issue(a, "must be blocked".into(), None, "509.1c", true),
                (None, None) => None,
            };
        }
        None
    }

    /// The declaration extending `fixed` with blocks among `allowed`, by creatures `fixed` does not have block, that keeps
    /// every attacker it blocks within its limits and obeys the most requirements (CR 509.1c), with how many; with `enough`,
    /// the first found that obeys at least that many. `None` when there is none. With `free_only`, blocks that cost something
    /// (CR 509.1d) are left out: no requirement is obeyed by paying a cost (CR 509.1c).
    ///
    /// Only blocks of an attacker a requirement can be obeyed through, or one `fixed` blocks, can obey a requirement or make up
    /// an attacker's fewest blockers, so only those are searched: the blockers with the most to obey first, their best blocks
    /// first. A branch is cut when it cannot beat the best found (or reach `enough`) - bounded by what the creatures still to
    /// come could obey, and by what the attackers' room for more blockers lets them obey, whichever is less - or when it leaves
    /// an attacker short of blockers no remaining creature can make up; the search ends once a declaration reaches the bound.
    pub fn solve(
        &self,
        allowed: &[(ObjectId, ObjectId)],
        fixed: &[(ObjectId, ObjectId)],
        enough: Option<u32>,
        free_only: bool,
    ) -> Option<(u32, Vec<(ObjectId, ObjectId)>)> {
        if self.capacities.iter().any(|c| c.1 != 1) {
            return self.solve_multiple(allowed, fixed, enough, free_only);
        }
        let usable = |b: ObjectId, a: ObjectId| !free_only || self.free(b, a);
        let mut targets: Vec<ObjectId> = Vec::new();
        let add = |x: ObjectId, targets: &mut Vec<ObjectId>| {
            if !targets.contains(&x) {
                targets.push(x);
            }
        };
        for &(_, a) in fixed {
            add(a, &mut targets);
        }
        for r in &self.required {
            for &(b, a) in allowed {
                if self.obeys(r, b, a) && usable(b, a) && !fixed.iter().any(|f| f.0 == b) {
                    add(a, &mut targets);
                }
            }
        }
        let limits: Vec<(u32, u32)> = targets.iter().map(|&a| self.limits_of(a)).collect();
        // The requirements on a blocker (Lure, "blocks this turn if able") count block by block; one on an attacker ("must be
        // blocked if able") once, however many creatures block it, so the search keeps how many blocks obey each.
        let own = |b: ObjectId, a: ObjectId| count_u32(self.required.iter().filter(|r| r.blocker.is_some() && self.obeys(r, b, a)).count());
        let on_attacker: Vec<ObjectId> = self.required.iter().filter(|r| r.blocker.is_none()).filter_map(|r| r.attacker).collect();
        let through: Vec<Vec<usize>> =
            targets.iter().map(|&t| (0..on_attacker.len()).filter(|&k| self.blocks_through(t, on_attacker[k])).collect()).collect();
        let meets: Vec<Vec<usize>> = (0..on_attacker.len()).map(|k| (0..targets.len()).filter(|&t| through[t].contains(&k)).collect()).collect();
        let mut counts = vec![0u32; targets.len()];
        let mut met = vec![0u32; on_attacker.len()];
        let mut fixed_gain = 0;
        for &(b, a) in fixed {
            fixed_gain += own(b, a);
            if let Some(t) = targets.iter().position(|&x| x == a) {
                counts[t] += 1;
                for &k in &through[t] {
                    if met[k] == 0 {
                        fixed_gain += 1;
                    }
                    met[k] += 1;
                }
            }
        }
        if counts.iter().zip(&limits).any(|(&n, &(_, most))| n > most) {
            return None;
        }
        // Each other creature that could block one of `targets`, with the attackers it may block and how many requirements on
        // it each block obeys, the most first.
        let mut blockers: Vec<(ObjectId, Vec<(usize, u32)>)> = Vec::new();
        for &(b, a) in allowed {
            if fixed.iter().any(|f| f.0 == b) || !usable(b, a) {
                continue;
            }
            let Some(t) = targets.iter().position(|&x| x == a) else { continue };
            let g = own(b, a);
            match blockers.iter_mut().find(|e| e.0 == b) {
                Some(e) => e.1.push((t, g)),
                None => blockers.push((b, vec![(t, g)])),
            }
        }
        for (_, options) in &mut blockers {
            options.sort_by_key(|&(t, g)| (std::cmp::Reverse(g), std::cmp::Reverse(through[t].len())));
        }
        blockers.sort_by_key(|(_, options)| std::cmp::Reverse(options.first().map_or(0, |o| o.1)));
        // What the blockers from each point on could obey, each its best block; how many of them could block each target; and,
        // for each target, the blockers that could block it by what that block obeys, the most first.
        let mut best_after = vec![0u32; blockers.len() + 1];
        let mut able_after = vec![vec![0u32; targets.len()]; blockers.len() + 1];
        for i in (0..blockers.len()).rev() {
            best_after[i] = best_after[i + 1] + blockers[i].1.first().map_or(0, |o| o.1);
            able_after[i] = able_after[i + 1].clone();
            for &(t, _) in &blockers[i].1 {
                able_after[i][t] += 1;
            }
        }
        let mut by_target: Vec<Vec<(usize, u32)>> = vec![Vec::new(); targets.len()];
        for (i, (_, options)) in blockers.iter().enumerate() {
            for &(t, g) in options {
                by_target[t].push((i, g));
            }
        }
        for list in &mut by_target {
            list.sort_by_key(|&(_, g)| std::cmp::Reverse(g));
        }

        struct Search<'a> {
            blockers: &'a [(ObjectId, Vec<(usize, u32)>)],
            limits: &'a [(u32, u32)],
            best_after: &'a [u32],
            able_after: &'a [Vec<u32>],
            by_target: &'a [Vec<(usize, u32)>],
            through: &'a [Vec<usize>],
            meets: &'a [Vec<usize>],
            enough: Option<u32>,
            counts: Vec<u32>,
            met: Vec<u32>,
            chosen: Vec<Option<usize>>,
            best: Option<(u32, Vec<Option<usize>>)>,
            ceiling: u32,
        }
        impl Search<'_> {
            /// The most the blockers from `i` on could still add: each its best block, but no attacker taking more of them
            /// than it has room for; and each requirement on an attacker not yet obeyed.
            fn bound(&self, i: usize) -> u32 {
                let by_blocker = self.best_after[i];
                let mut by_room = 0u32;
                for (t, list) in self.by_target.iter().enumerate() {
                    let mut room = self.limits[t].1 - self.counts[t];
                    for &(b, g) in list {
                        if room == 0 || g == 0 {
                            break;
                        }
                        if b >= i {
                            by_room += g;
                            room -= 1;
                        }
                    }
                    if by_room >= by_blocker {
                        break;
                    }
                }
                // A requirement on an attacker not yet obeyed counts only where a creature still to come may block an attacker
                // that obeys it, with room for one more blocker.
                let open = (0..self.met.len())
                    .filter(|&k| {
                        self.met[k] == 0 && self.meets[k].iter().any(|&t| self.counts[t] < self.limits[t].1 && self.able_after[i][t] > 0)
                    })
                    .count();
                by_blocker.min(by_room) + count_u32(open)
            }

            fn done(&self) -> bool {
                self.best.as_ref().is_some_and(|(n, _)| self.enough.is_some() || *n >= self.ceiling)
            }

            fn go(&mut self, i: usize, obeyed: u32) {
                // An attacker blocked by fewer than it may be needs that many more of the creatures still to come.
                if self.counts.iter().zip(self.limits).enumerate().any(|(t, (&n, &(least, _)))| n > 0 && n < least && self.able_after[i][t] < least - n) {
                    return;
                }
                // A whole declaration is judged by what it obeys, nothing more.
                if i == self.blockers.len() {
                    let better = match self.enough {
                        Some(n) => obeyed >= n,
                        None => self.best.as_ref().is_none_or(|(n, _)| obeyed > *n),
                    };
                    if better {
                        self.best = Some((obeyed, self.chosen.clone()));
                    }
                    return;
                }
                let reach = obeyed + self.bound(i);
                match self.enough {
                    Some(n) if reach < n => return,
                    None if self.best.as_ref().is_some_and(|(n, _)| reach <= *n) => return,
                    _ => {}
                }
                for k in 0..self.blockers[i].1.len() {
                    let (t, gain) = self.blockers[i].1[k];
                    if self.counts[t] >= self.limits[t].1 {
                        continue;
                    }
                    let mut more = gain;
                    for &r in &self.through[t] {
                        if self.met[r] == 0 {
                            more += 1;
                        }
                        self.met[r] += 1;
                    }
                    self.counts[t] += 1;
                    self.chosen[i] = Some(t);
                    self.go(i + 1, obeyed + more);
                    self.counts[t] -= 1;
                    for &r in &self.through[t] {
                        self.met[r] -= 1;
                    }
                    if self.done() {
                        return;
                    }
                }
                self.chosen[i] = None;
                self.go(i + 1, obeyed);
            }
        }
        let mut search = Search {
            blockers: &blockers,
            limits: &limits,
            best_after: &best_after,
            able_after: &able_after,
            by_target: &by_target,
            through: &through,
            meets: &meets,
            enough: enough.map(|n| n.saturating_sub(fixed_gain)),
            counts,
            met,
            chosen: vec![None; blockers.len()],
            best: None,
            ceiling: 0,
        };
        search.ceiling = search.bound(0);
        search.go(0, 0);
        let (obeyed, chosen) = search.best?;
        let mut declaration = fixed.to_vec();
        declaration.extend(blockers.iter().zip(chosen).filter_map(|((b, _), t)| t.map(|t| (*b, targets[t]))));
        Some((obeyed + fixed_gain, declaration))
    }

    /// The same completion search when one creature can be declared against several attackers. Requirements count
    /// once each, however many of the creature's blocks obey them. The ordinary one-attacker path above stays unchanged.
    fn solve_multiple(
        &self,
        allowed: &[(ObjectId, ObjectId)],
        fixed: &[(ObjectId, ObjectId)],
        enough: Option<u32>,
        free_only: bool,
    ) -> Option<(u32, Vec<(ObjectId, ObjectId)>)> {
        if self.issue(allowed, fixed).is_some_and(|(_, extendable)| !extendable) { return None; }
        let mut targets: Vec<ObjectId> = fixed.iter().map(|p| p.1).collect();
        for r in &self.required {
            for &(b, a) in allowed {
                if self.obeys(r, b, a) && (!free_only || self.free(b, a)) && !targets.contains(&a) {
                    targets.push(a);
                }
            }
        }
        let mut edges = Vec::new();
        for &(b, a) in allowed {
            if targets.contains(&a) && !fixed.contains(&(b, a)) && !edges.contains(&(b, a))
                && (!free_only || self.free(b, a))
            {
                edges.push((b, a));
            }
        }
        edges.sort_by_key(|&(b, a)| std::cmp::Reverse(self.required.iter().filter(|r| self.obeys(r, b, a)).count()));

        struct Search<'a> {
            rules: &'a BlockRequirements,
            edges: &'a [(ObjectId, ObjectId)],
            targets: &'a [ObjectId],
            chosen: Vec<(ObjectId, ObjectId)>,
            enough: Option<u32>,
            best: Option<(u32, Vec<(ObjectId, ObjectId)>)>,
        }
        impl Search<'_> {
            fn fits(&self, (b, a): (ObjectId, ObjectId)) -> bool {
                count_u32(self.chosen.iter().filter(|p| p.0 == b).count()) < self.rules.capacity_of(b)
                    && count_u32(self.chosen.iter().filter(|p| p.1 == a).count()) < self.rules.limits_of(a).1
            }
            fn legal_counts(&self) -> bool {
                self.targets.iter().all(|&a| {
                    let n = count_u32(self.chosen.iter().filter(|p| p.1 == a).count());
                    let (least, most) = self.rules.limits_of(a);
                    n == 0 || (least <= n && n <= most)
                })
            }
            fn go(&mut self, at: usize) -> bool {
                #[cfg(test)] MULTI_SEARCH_NODES.with(|nodes| nodes.set(nodes.get() + 1));
                let score = self.rules.obeyed(&self.chosen);
                if self.legal_counts() && self.enough.is_none_or(|n| score >= n)
                    && self.best.as_ref().is_none_or(|(n, _)| score > *n)
                {
                    self.best = Some((score, self.chosen.clone()));
                    if self.enough.is_some() || score == count_u32(self.rules.required.len()) { return true; }
                }
                // Each remaining declaration uses one slot of its blocker. For each blocker, add the largest
                // possible gains of its remaining slots, counting only requirements not yet obeyed. A requirement
                // satisfied by several future edges is counted several times, so the sum can overestimate but never
                // underestimate what any completion can achieve (including band-expanded requirements).
                let unmet: Vec<_> = self.rules.required.iter().filter(|r| {
                    !self.chosen.iter().any(|&(b, a)| self.rules.obeys(r, b, a))
                }).collect();
                let mut blockers = Vec::new();
                for &(b, _) in &self.edges[at..] {
                    if !blockers.contains(&b) { blockers.push(b); }
                }
                let mut bound = score;
                for blocker in blockers {
                    let used = count_u32(self.chosen.iter().filter(|p| p.0 == blocker).count());
                    let slots = self.rules.capacity_of(blocker).saturating_sub(used) as usize;
                    let mut gains: Vec<u32> = self.edges[at..].iter().filter_map(|&edge @ (b, a)| {
                        (b == blocker && self.fits(edge)).then(|| count_u32(unmet.iter().filter(|r| self.rules.obeys(r, b, a)).count()))
                    }).collect();
                    gains.sort_unstable_by(|a, b| b.cmp(a));
                    for gain in gains.into_iter().take(slots) { bound = bound.saturating_add(gain); }
                }
                bound = bound.min(count_u32(self.rules.required.len()));
                if self.enough.is_some_and(|n| bound < n)
                    || self.best.as_ref().is_some_and(|(n, _)| bound <= *n) { return false; }
                // Each occupied attacker must still be able to reach its minimum. Distinct (blocker, attacker) edges
                // ensure no creature counts twice toward a menace requirement.
                for &a in self.targets {
                    let n = count_u32(self.chosen.iter().filter(|p| p.1 == a).count());
                    let least = self.rules.limits_of(a).0;
                    if n > 0 && n < least {
                        let more = count_u32(self.edges[at..].iter().filter(|&&edge| edge.1 == a && self.fits(edge)).count());
                        if n + more < least { return false; }
                    }
                }
                let Some(&edge) = self.edges.get(at) else { return false };
                if self.fits(edge) {
                    self.chosen.push(edge);
                    let done = self.go(at + 1);
                    self.chosen.pop();
                    if done { return true; }
                }
                self.go(at + 1)
            }
        }
        let mut search = Search { rules: self, edges: &edges, targets: &targets, chosen: fixed.to_vec(), enough, best: None };
        search.go(0);
        search.best
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{BlockCost, RequiredBlock};

    const A: ObjectId = ObjectId(1);
    const B: ObjectId = ObjectId(2);
    const X: ObjectId = ObjectId(10);
    const Y: ObjectId = ObjectId(11);

    fn blocks(blocker: ObjectId) -> RequiredBlock {
        RequiredBlock { blocker: Some(blocker), attacker: None }
    }

    #[test]
    fn an_additional_block_allowance_allows_two_distinct_attackers() {
        let allowed = [(X, A), (X, B)];
        let r = BlockRequirements { capacities: vec![(X, 2)], ..Default::default() };
        assert!(r.judge(&allowed, &allowed, false).complete);
        assert_eq!(r.judge(&allowed, &[(X, A)], true).addable, [(X, B)]);
        assert!(!r.judge(&allowed, &[(X, A), (X, A)], false).complete);
        assert!(!BlockRequirements::default().judge(&allowed, &allowed, false).complete);
    }

    #[test]
    fn one_blocker_can_obey_two_different_lures_without_counting_one_requirement_twice() {
        let allowed = [(X, A), (X, B)];
        let mut r = BlockRequirements { capacities: vec![(X, 2)],
            required: vec![blocks(X), lure(X, A), lure(X, B)], ..Default::default() };
        let (most, suggested) = r.solve(&allowed, &[], None, true).unwrap();
        assert_eq!(most, 3);
        assert_eq!(r.obeyed(&suggested), 3);
        r.most = most;
        assert!(!r.judge(&allowed, &[(X, A)], false).complete);
        assert_eq!(r.judge(&allowed, &[(X, A)], true).addable, [(X, B)]);
        assert!(r.judge(&allowed, &allowed, false).complete);
    }

    #[test]
    fn additional_blocks_still_need_distinct_creatures_to_satisfy_menace() {
        let allowed = [(X, A), (X, B), (Y, A), (Y, B)];
        let mut r = BlockRequirements { capacities: vec![(X, 2), (Y, 2)],
            required: vec![RequiredBlock { blocker: None, attacker: Some(A) },
                RequiredBlock { blocker: None, attacker: Some(B) }],
            limits: vec![(A, 2, 2), (B, 2, 2)], ..Default::default() };
        let (most, suggested) = r.solve(&allowed, &[], None, true).unwrap();
        assert_eq!(most, 2);
        assert_eq!(suggested.len(), 4);
        r.most = most;
        assert!(!r.judge(&allowed, &[(X, A), (X, B)], false).complete);
        assert!(r.judge(&allowed, &[(X, A)], true).addable.contains(&(X, B)));
        assert!(r.judge(&allowed, &allowed, false).complete);
    }

    #[test]
    fn blocking_a_band_expands_requirements_without_consuming_extra_declarations() {
        let allowed = [(X, A), (X, B)];
        let mut r = BlockRequirements { capacities: vec![(X, 2)], bands: vec![(A, 1), (B, 1)],
            required: vec![lure(X, A), lure(X, B)], ..Default::default() };
        let (most, _) = r.solve(&allowed, &[], None, true).unwrap();
        assert_eq!(most, 2);
        r.most = most;
        assert!(r.judge(&allowed, &[(X, A)], false).complete);
    }

    #[test]
    fn dense_lures_with_finite_multiple_blockers_finish() {
        let attackers: Vec<_> = (0..8).map(|n| ObjectId(100 + n)).collect();
        let blockers: Vec<_> = (0..8).map(|n| ObjectId(200 + n)).collect();
        let allowed: Vec<_> = blockers.iter().flat_map(|&b| attackers.iter().map(move |&a| (b, a))).collect();
        let required = allowed.iter().map(|&(b, a)| lure(b, a)).collect();
        let r = BlockRequirements { capacities: blockers.iter().map(|&b| (b, 2)).collect(), required, ..Default::default() };
        MULTI_SEARCH_NODES.with(|nodes| nodes.set(0));
        let (score, chosen) = r.solve(&allowed, &[], None, true).unwrap();
        let nodes = MULTI_SEARCH_NODES.with(|nodes| nodes.get());
        assert!(nodes <= 256, "dense declaration visited {nodes} search nodes");
        eprintln!("dense finite declaration: {nodes} search nodes");
        assert_eq!(score, 16);
        assert_eq!(chosen.len(), 16);
        assert_eq!(r.obeyed(&chosen), 16);
    }

    #[test]
    fn multiple_block_completion_matches_exhaustive_small_declarations() {
        let all = [(X, A), (X, B), (Y, A), (Y, B)];
        let required = [vec![blocks(X)], vec![lure(X, A), lure(X, B)],
            vec![RequiredBlock { blocker: None, attacker: Some(A) }, RequiredBlock { blocker: None, attacker: Some(B) }],
            vec![blocks(X), blocks(X), lure(Y, B)]];
        for allowed_mask in 0u32..16 {
            let allowed: Vec<_> = all.iter().enumerate().filter(|(i, _)| allowed_mask & (1 << i) != 0).map(|(_, p)| *p).collect();
            for (cx, cy) in [(1, 1), (2, 1), (1, 2), (2, 2), (0, 2)] {
                for (ma, mb) in [(1, 1), (2, 1), (1, 2), (2, 2)] {
                    for needs in &required {
                        for paid in [false, true] {
                            let mut r = BlockRequirements { capacities: vec![(X, cx), (Y, cy)],
                                limits: vec![(A, ma, 2), (B, mb, 2)], required: needs.clone(), ..Default::default() };
                            if paid { r.costs.push(BlockCost { blocker: X, attacker: A,
                                cost: ManaCost { generic: 1, ..ManaCost::ZERO }, source: A }); }
                            let declarations: Vec<Vec<_>> = (0..16).map(|mask| all.iter().enumerate()
                                .filter(|(i, _)| mask & (1 << i) != 0).map(|(_, p)| *p).collect()).collect();
                            let valid = |decl: &Vec<(ObjectId, ObjectId)>| {
                                decl.iter().all(|p| allowed.contains(p))
                                    && [(X, cx), (Y, cy)].iter().all(|&(b, cap)| decl.iter().filter(|p| p.0 == b).count() <= cap as usize)
                                    && [(A, ma), (B, mb)].iter().all(|&(a, min)| {
                                        let n = decl.iter().filter(|p| p.1 == a).count(); n == 0 || n >= min as usize
                                    })
                            };
                            // This oracle checks requirements directly instead of using solve or judge.
                            let score = |decl: &Vec<(ObjectId, ObjectId)>| needs.iter().filter(|need| decl.iter()
                                .any(|&(b, a)| need.blocker.is_none_or(|id| id == b) && need.attacker.is_none_or(|id| id == a))).count() as u32;
                            let best = declarations.iter().filter(|d| valid(d) && (!paid || !d.contains(&(X, A))))
                                .map(score).max().unwrap();
                            let found = r.solve(&allowed, &[], None, true).unwrap();
                            assert_eq!(found.0, best, "allowed={allowed_mask} caps={cx},{cy} minimum={ma},{mb} needs={needs:?} paid={paid}");
                            assert!(valid(&found.1));
                            r.most = best;
                            for partial in &declarations {
                                let judged = r.judge(&allowed, partial, true);
                                assert_eq!(judged.complete, valid(partial) && score(partial) >= best);
                                let expected: Vec<_> = allowed.iter().copied().filter(|p| !partial.contains(p)
                                    && declarations.iter().any(|d| valid(d) && score(d) >= best
                                        && partial.iter().all(|p| d.contains(p)) && d.contains(p))).collect();
                                assert_eq!(judged.addable, expected, "partial={partial:?} caps={cx},{cy} needs={needs:?}");
                            }
                        }
                    }
                }
            }
        }
    }

    fn lure(blocker: ObjectId, attacker: ObjectId) -> RequiredBlock {
        RequiredBlock { blocker: Some(blocker), attacker: Some(attacker) }
    }

    fn rules(required: Vec<RequiredBlock>, limits: Vec<(ObjectId, u32, u32)>, allowed: &[(ObjectId, ObjectId)]) -> BlockRequirements {
        let mut r = BlockRequirements { required, limits, ..Default::default() };
        if let Some((most, suggested)) = r.solve(allowed, &[], None, true) {
            r.most = most;
            r.suggested = if most > 0 { suggested } else { Vec::new() };
        }
        r
    }

    /// A requirement to block is one requirement, obeyed by blocking any attacker the creature may block.
    #[test]
    fn a_creature_that_must_block_obeys_by_blocking_any_attacker() {
        let allowed = [(X, A), (X, B)];
        let r = rules(vec![blocks(X)], vec![(A, 1, u32::MAX), (B, 1, u32::MAX)], &allowed);
        assert_eq!(r.most, 1);
        assert!(r.judge(&allowed, &[(X, A)], false).complete && r.judge(&allowed, &[(X, B)], false).complete);
        let none = r.judge(&allowed, &[], true);
        assert!(!none.complete);
        assert_eq!(none.issue.as_ref().map(|i| (i.creature, i.rule)), Some((X, "509.1c")));
        assert_eq!(none.addable, vec![(X, A), (X, B)]);
    }

    /// A required blocker of a menace creature alone obeys nothing: with a second creature able to join it, the requirement
    /// is obeyed by both blocking; adding the first block is addable though the declaration with it alone is not legal.
    #[test]
    fn menace_needs_a_second_blocker_for_the_requirement() {
        let allowed = [(X, A), (Y, A)];
        let r = rules(vec![blocks(X)], vec![(A, 2, u32::MAX)], &allowed);
        assert_eq!(r.most, 1);
        let one = r.judge(&allowed, &[(X, A)], true);
        assert!(!one.complete && one.addable == vec![(Y, A)]);
        assert!(r.judge(&allowed, &[(X, A), (Y, A)], false).complete);
        // With no second creature, nothing is required.
        let alone = rules(vec![blocks(X)], vec![(A, 2, u32::MAX)], &[(X, A)]);
        assert_eq!(alone.most, 0);
        assert!(alone.judge(&[(X, A)], &[], false).complete);
        assert!(alone.judge(&[(X, A)], &[], true).addable.is_empty(), "no legal declaration has X blocking alone");
    }

    /// A requirement to block any attacker and a Lure-style one to block a given attacker are counted together.
    #[test]
    fn requirements_to_block_and_to_block_an_attacker_add_up() {
        let allowed = [(X, A), (X, B), (Y, A)];
        let r = rules(vec![blocks(X), lure(Y, A), lure(X, A)], vec![(A, 1, 1), (B, 1, u32::MAX)], &allowed);
        // A can be blocked by one creature: Y on A and X on B obeys two, as does X on A alone (two of X's) - the most is 2.
        assert_eq!(r.most, 2);
        assert!(r.judge(&allowed, &[(Y, A), (X, B)], false).complete);
        assert!(r.judge(&allowed, &[(X, A)], false).complete);
        assert!(!r.judge(&allowed, &[(Y, A)], false).complete);
        assert!(!r.judge(&allowed, &[(X, B)], false).complete);
        let over = r.judge(&allowed, &[(X, A), (Y, A)], true);
        assert!(!over.complete && over.addable.is_empty() && over.issue.is_some_and(|i| i.creature == A));
    }

    /// Requirements count one per effect: two on X (two Academic Disputes) and one on Y, with an attacker only one creature may
    /// block, make X's block obey two and Y's one - only X blocking is legal.
    #[test]
    fn two_requirements_on_one_creature_count_twice() {
        let allowed = [(X, A), (Y, A)];
        let r = rules(vec![blocks(X), blocks(X), blocks(Y)], vec![(A, 1, 1)], &allowed);
        assert_eq!(r.most, 2);
        assert!(r.judge(&allowed, &[(X, A)], false).complete);
        assert!(!r.judge(&allowed, &[(Y, A)], false).complete);
    }

    /// "Must be blocked if able" (a requirement on an attacker) is obeyed once, by any creature blocking it, however many do.
    #[test]
    fn a_requirement_on_an_attacker_is_obeyed_by_any_blocker_once() {
        let allowed = [(X, A), (Y, A), (X, B), (Y, B)];
        let must_be_blocked = RequiredBlock { blocker: None, attacker: Some(A) };
        let r = rules(vec![must_be_blocked], vec![(A, 1, u32::MAX), (B, 1, u32::MAX)], &allowed);
        assert_eq!(r.most, 1);
        assert!(!r.judge(&allowed, &[(X, B)], false).complete);
        assert!(r.judge(&allowed, &[(Y, A)], false).complete && r.judge(&allowed, &[(X, A), (Y, A)], false).complete);
        let none = r.judge(&allowed, &[], false);
        assert_eq!(none.issue.map(|i| (i.creature, i.words)), Some((A, "must be blocked".to_string())));
    }

    /// A block that costs something obeys no requirement the player must obey (CR 509.1c): with X's only block costing {2},
    /// X need not block, and blocking and paying is still legal, at its cost.
    #[test]
    fn a_requirement_is_never_obeyed_by_paying() {
        let allowed = [(X, A)];
        let mut r = BlockRequirements { required: vec![blocks(X)], ..Default::default() };
        r.costs = vec![BlockCost { blocker: X, attacker: A, cost: ManaCost { generic: 2, ..ManaCost::ZERO }, source: B }];
        r.most = r.solve(&allowed, &[], None, true).map_or(0, |s| s.0);
        assert_eq!(r.most, 0);
        assert!(r.judge(&allowed, &[], false).complete);
        let paid = r.judge(&allowed, &[(X, A)], false);
        assert!(paid.complete && paid.cost.generic == 2);
    }

    /// Lured attackers that can't be blocked by more than one creature each, and every creature able to block any of them: the
    /// most is one per attacker, found and every addable block judged at once, at a size hand play can build (twelve such
    /// attackers, twenty-four creatures), where a search bounded only by what each creature could obey looks at every way of
    /// choosing a blocker for each attacker.
    #[test]
    fn capacity_bounds_the_search_at_practical_scale() {
        let attackers: Vec<ObjectId> = (1..=12).map(ObjectId).collect();
        let creatures: Vec<ObjectId> = (100..124).map(ObjectId).collect();
        let allowed: Vec<(ObjectId, ObjectId)> = creatures.iter().flat_map(|&b| attackers.iter().map(move |&a| (b, a))).collect();
        let required: Vec<RequiredBlock> = allowed.iter().map(|&(b, a)| lure(b, a)).collect();
        let limits = attackers.iter().map(|&a| (a, 1, 1)).collect();
        let started = std::time::Instant::now();
        let r = rules(required, limits, &allowed);
        assert_eq!(r.most, 12);
        let none = r.judge(&allowed, &[], true);
        assert_eq!(none.addable.len(), allowed.len());
        let half: Vec<(ObjectId, ObjectId)> = (0..6).map(|i| (creatures[i], attackers[i])).collect();
        let verdict = r.judge(&allowed, &half, true);
        assert!(!verdict.complete);
        assert_eq!(verdict.addable.len(), 18 * 6, "each creature not blocking, onto each attacker not yet blocked");
        assert!(started.elapsed() < std::time::Duration::from_secs(5), "took {:?}", started.elapsed());
    }

    /// Two attackers that must be blocked if able, one creature able to block either: it blocks one and obeys one, so no
    /// blocks at all is refused (the most is 1, not 0).
    #[test]
    fn requirements_on_two_attackers_one_creature_can_meet_one() {
        let allowed = [(X, A), (X, B)];
        let on = |a| RequiredBlock { blocker: None, attacker: Some(a) };
        let r = rules(vec![on(A), on(B)], vec![(A, 1, u32::MAX), (B, 1, u32::MAX)], &allowed);
        assert_eq!(r.most, 1);
        let none = r.judge(&allowed, &[], true);
        assert!(!none.complete);
        assert_eq!(none.addable, vec![(X, A), (X, B)]);
        assert!(r.judge(&allowed, &[(X, A)], false).complete && r.judge(&allowed, &[(X, B)], false).complete);
    }

    /// Every way of blocking, over small random boards (allowed blocks, fewest and most blockers, bands, costs, and
    /// requirements of every kind, some on the same creature twice): `solve`'s most is the most any free legal declaration
    /// obeys, a declaration is complete exactly when it is legal, and `addable` offers exactly the blocks some legal
    /// declaration extending it has - never one that none does.
    #[test]
    fn the_verdict_matches_every_declaration_on_small_boards() {
        let mut seed: u64 = 0x9E37_79B9_7F4A_7C15;
        let mut next = |n: u64| {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            seed % n
        };
        for _ in 0..400 {
            let blockers: Vec<ObjectId> = (0..1 + next(4)).map(|i| ObjectId(10 + i as u32)).collect();
            let attackers: Vec<ObjectId> = (0..1 + next(3)).map(|i| ObjectId(1 + i as u32)).collect();
            let allowed: Vec<(ObjectId, ObjectId)> =
                blockers.iter().flat_map(|&b| attackers.iter().map(move |&a| (b, a))).filter(|_| true).collect::<Vec<_>>();
            let allowed: Vec<(ObjectId, ObjectId)> = allowed.into_iter().filter(|_| next(4) != 0).collect();
            let limits: Vec<(ObjectId, u32, u32)> = attackers
                .iter()
                .map(|&a| {
                    let least = 1 + u32::from(next(4) == 0);
                    let most = match next(3) {
                        0 => 1,
                        1 => 2,
                        _ => u32::MAX,
                    };
                    (a, least, most)
                })
                .collect();
            let mut required = Vec::new();
            for _ in 0..next(5) {
                let b = blockers[next(blockers.len() as u64) as usize];
                let a = attackers[next(attackers.len() as u64) as usize];
                required.push(match next(3) {
                    0 => RequiredBlock { blocker: Some(b), attacker: None },
                    1 => RequiredBlock { blocker: None, attacker: Some(a) },
                    _ => RequiredBlock { blocker: Some(b), attacker: Some(a) },
                });
            }
            let bands = if attackers.len() > 1 && next(4) == 0 { vec![(attackers[0], 1), (attackers[1], 1)] } else { Vec::new() };
            let costs: Vec<BlockCost> = allowed
                .iter()
                .filter(|_| next(6) == 0)
                .map(|&(b, a)| BlockCost { blocker: b, attacker: a, cost: ManaCost { generic: 1, ..ManaCost::ZERO }, source: ObjectId(99) })
                .collect();
            let mut r = BlockRequirements { required, bands, limits, costs, ..Default::default() };
            // Every declaration: each blocker blocks one allowed attacker or none.
            let mut all: Vec<Vec<(ObjectId, ObjectId)>> = vec![Vec::new()];
            for &b in &blockers {
                let mut more = Vec::new();
                for d in &all {
                    more.push(d.clone());
                    for &(bb, a) in allowed.iter().filter(|p| p.0 == b) {
                        let mut e = d.clone();
                        e.push((bb, a));
                        more.push(e);
                    }
                }
                all = more;
            }
            let plain = r.clone();
            let fits = |d: &[(ObjectId, ObjectId)]| {
                attackers.iter().all(|&a| {
                    let n = count_u32(d.iter().filter(|x| x.1 == a).count());
                    let (least, most) = plain.limits_of(a);
                    n == 0 || (least <= n && n <= most)
                })
            };
            let most = all.iter().filter(|d| fits(d) && d.iter().all(|&(b, a)| plain.free(b, a))).map(|d| plain.obeyed(d)).max().unwrap_or(0);
            r.most = r.solve(&allowed, &[], None, true).map_or(0, |s| s.0);
            assert_eq!(r.most, most, "{r:?} over {allowed:?}");
            let legal = |d: &[(ObjectId, ObjectId)]| fits(d) && plain.obeyed(d) >= most;
            for d in all.iter().filter(|d| d.len() <= 2) {
                let verdict = r.judge(&allowed, d, true);
                assert_eq!(verdict.complete, legal(d), "{d:?} under {r:?}");
                let wanted: Vec<(ObjectId, ObjectId)> = allowed
                    .iter()
                    .copied()
                    .filter(|&(b, a)| {
                        !d.iter().any(|x| x.0 == b)
                            && all.iter().any(|l| l.contains(&(b, a)) && d.iter().all(|x| l.contains(x)) && legal(l))
                    })
                    .collect();
                assert_eq!(verdict.addable, wanted, "addable to {d:?} under {r:?} over {allowed:?}");
            }
        }
    }
}
