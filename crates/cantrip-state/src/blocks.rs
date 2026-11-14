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
//! A declaration is a list of blocks, each one creature blocking one attacker. A creature appears in as many as it can block
//! (`BlockRequirements::capacity_of`: one, or more for "can block an additional creature" and "can block any number of
//! creatures", CR 509.1a), each attacker once.
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
                    count_u32(blocks.iter().filter(|x| x.0 == b).count()) < self.capacity_of(b)
                        && !blocks.iter().any(|x| x.0 == b && self.blocks_through(x.1, a))
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
    /// mend it: not past a block that is not allowed, a creature blocking more creatures than it can or one creature twice,
    /// or an attacker over its most.
    fn issue(&self, allowed: &[(ObjectId, ObjectId)], blocks: &[(ObjectId, ObjectId)]) -> Option<(BlockIssue, bool)> {
        let issue = |creature, words: String, other, rule, mendable| Some((BlockIssue { creature, words, other, rule }, mendable));
        for (i, &(b, a)) in blocks.iter().enumerate() {
            // CR 509.1a: one creature for each blocker to block, or as many as it can block (`BlockCapacity`).
            let capacity = self.capacity_of(b);
            if capacity != u32::MAX && count_u32(blocks[..=i].iter().filter(|x| x.0 == b).count()) > capacity {
                let words = if capacity == 1 { "can block only one creature".into() } else { format!("can block only {}", creatures(capacity)) };
                return issue(b, words, None, "509.1a", false);
            }
            // Each attacker it blocks is chosen once, and a creature blocking one member of a band already blocks the others
            // (CR 702.22h).
            if blocks[..i].iter().any(|x| x.0 == b && self.blocks_through(x.1, a)) {
                return issue(b, "already blocks".into(), Some(a), "509.1a", false);
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
        let has = |id| blocks.iter().any(|p| p.0 == id);
        if let Some(g) = self.groups.iter().find(|g| !g.allows(has)) {
            return issue(g.subjects.iter().copied().find(|&id| has(id)).unwrap_or(ObjectId::NONE), "can't block with this group".into(), None, "509.1b", g.count(has) <= g.at_most);
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

    /// The declaration extending `fixed` with blocks among `allowed`, each creature blocking no more attackers than it can (one,
    /// or its `capacity_of`, `fixed`'s blocks counted), that keeps every attacker it blocks within its limits and obeys the most
    /// requirements (CR 509.1c), with how many; with `enough`, the first found that obeys at least that many. `None` when there
    /// is none. With `free_only`, blocks that cost something (CR 509.1d) are left out: no requirement is obeyed by paying a cost
    /// (CR 509.1c).
    ///
    /// Only blocks of an attacker a requirement can be obeyed through, or one `fixed` blocks, can obey a requirement or make up
    /// an attacker's fewest blockers, so only those are searched: the blockers with the most to obey first, their best blocks
    /// first, each choosing which of them to block (a creature that can block several, any set of them up to its capacity). A
    /// branch is cut when it cannot beat the best found (or reach `enough`) - bounded by what the creatures still to come could
    /// obey, and by what the attackers' room for more blockers lets them obey, whichever is less - or when it leaves an attacker
    /// short of blockers no remaining creature can make up; the search ends once a declaration reaches the bound.
    pub fn solve(
        &self,
        allowed: &[(ObjectId, ObjectId)],
        fixed: &[(ObjectId, ObjectId)],
        enough: Option<u32>,
        free_only: bool,
    ) -> Option<(u32, Vec<(ObjectId, ObjectId)>)> {
        let usable = |b: ObjectId, a: ObjectId| !free_only || self.free(b, a);
        // How many more attackers a creature can block past those `fixed` has it block (none past its capacity), and whether
        // it may block `a` as well: not an attacker it blocks already, as declared or through a band (CR 702.22h).
        let room = |b: ObjectId| self.capacity_of(b).saturating_sub(count_u32(fixed.iter().filter(|f| f.0 == b).count()));
        let open = |b: ObjectId, a: ObjectId| room(b) > 0 && !fixed.iter().any(|f| f.0 == b && self.blocks_through(f.1, a));
        let mut targets: Vec<ObjectId> = Vec::new();
        let add = |x: ObjectId, targets: &mut Vec<ObjectId>| {
            if !targets.contains(&x) {
                targets.push(x);
            }
        };
        for &(_, a) in fixed {
            add(a, &mut targets);
        }
        if !self.groups.is_empty() {
            for &(b,a) in allowed { if usable(b,a) && open(b,a) { add(a,&mut targets); } }
        }
        for r in &self.required {
            for &(b, a) in allowed {
                if self.obeys(r, b, a) && usable(b, a) && open(b, a) {
                    add(a, &mut targets);
                }
            }
        }
        let limits: Vec<(u32, u32)> = targets.iter().map(|&a| self.limits_of(a)).collect();
        // A requirement to block a given attacker (Lure, "blocks each attacking creature if able") counts block by block; one
        // on a blocker to block whichever attacker ("blocks this turn if able") once for the creature, however many it blocks;
        // and one on an attacker ("must be blocked if able") once, however many creatures block it, so the search keeps how
        // many blocks obey each.
        let own = |b: ObjectId, a: ObjectId| {
            count_u32(self.required.iter().filter(|r| r.blocker == Some(b) && r.attacker.is_some() && self.obeys(r, b, a)).count())
        };
        let own_any = |b: ObjectId| count_u32(self.required.iter().filter(|r| r.blocker == Some(b) && r.attacker.is_none()).count());
        let on_attacker: Vec<ObjectId> = self.required.iter().filter(|r| r.blocker.is_none()).filter_map(|r| r.attacker).collect();
        let through: Vec<Vec<usize>> =
            targets.iter().map(|&t| (0..on_attacker.len()).filter(|&k| self.blocks_through(t, on_attacker[k])).collect()).collect();
        let meets: Vec<Vec<usize>> = (0..on_attacker.len()).map(|k| (0..targets.len()).filter(|&t| through[t].contains(&k)).collect()).collect();
        let mut counts = vec![0u32; targets.len()];
        let mut met = vec![0u32; on_attacker.len()];
        let mut fixed_gain = 0;
        for (i, &(b, a)) in fixed.iter().enumerate() {
            fixed_gain += own(b, a);
            if !fixed[..i].iter().any(|f| f.0 == b) {
                fixed_gain += own_any(b);
            }
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
        // Each creature that could block one more of `targets`, with the attackers it may block and how many requirements to
        // block that attacker each block obeys, the most first; how many more it can block; and how many requirements to block
        // whichever attacker its first block obeys (none for one `fixed` has blocking already).
        let mut blockers: Vec<Blocker> = Vec::new();
        for &(b, a) in allowed {
            if !usable(b, a) || !open(b, a) {
                continue;
            }
            let Some(t) = targets.iter().position(|&x| x == a) else { continue };
            let g = own(b, a);
            match blockers.iter_mut().find(|e| e.id == b) {
                Some(e) => e.options.push((t, g)),
                None => blockers.push(Blocker {
                    id: b,
                    options: vec![(t, g)],
                    room: room(b),
                    any: if fixed.iter().any(|f| f.0 == b) { 0 } else { own_any(b) },
                }),
            }
        }
        for e in &mut blockers {
            e.options.sort_by_key(|&(t, g)| (std::cmp::Reverse(g), std::cmp::Reverse(through[t].len())));
        }
        blockers.sort_by_key(|e| std::cmp::Reverse(e.any + e.options.first().map_or(0, |o| o.1)));
        // What the blockers from each point on could obey, each its best blocks; how many of them could block each target; and,
        // for each target, the blockers that could block it by what that block obeys, the most first.
        let mut best_after = vec![0u32; blockers.len() + 1];
        let mut able_after = vec![vec![0u32; targets.len()]; blockers.len() + 1];
        for i in (0..blockers.len()).rev() {
            let e = &blockers[i];
            best_after[i] = best_after[i + 1] + e.any + e.options.iter().take(e.room as usize).map(|o| o.1).sum::<u32>();
            able_after[i] = able_after[i + 1].clone();
            for &(t, _) in &e.options {
                able_after[i][t] += 1;
            }
        }
        let bands: Vec<u32> = targets.iter().map(|&t| self.bands.iter().find(|b| b.0 == t).map_or(0, |b| b.1)).collect();
        let mut by_target: Vec<Vec<(usize, u32)>> = vec![Vec::new(); targets.len()];
        for (i, e) in blockers.iter().enumerate() {
            for &(t, g) in &e.options {
                by_target[t].push((i, e.any + g));
            }
        }
        for list in &mut by_target {
            list.sort_by_key(|&(_, g)| std::cmp::Reverse(g));
        }

        struct Search<'a> {
            blockers: &'a [Blocker],
            groups: &'a [crate::CombatGroup],
            fixed: &'a [(ObjectId,ObjectId)],
            limits: &'a [(u32, u32)],
            best_after: &'a [u32],
            able_after: &'a [Vec<u32>],
            by_target: &'a [Vec<(usize, u32)>],
            through: &'a [Vec<usize>],
            meets: &'a [Vec<usize>],
            /// The band of each target, 0 for none.
            bands: &'a [u32],
            enough: Option<u32>,
            counts: Vec<u32>,
            met: Vec<u32>,
            /// The blocks chosen so far, each a blocker's index and a target's.
            chosen: Vec<(usize, usize)>,
            best: Option<(u32, Vec<(usize, usize)>)>,
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
                #[cfg(test)] MULTI_SEARCH_NODES.with(|nodes| nodes.set(nodes.get() + 1));
                // An attacker blocked by fewer than it may be needs that many more of the creatures still to come.
                if self.counts.iter().zip(self.limits).enumerate().any(|(t, (&n, &(least, _)))| n > 0 && n < least && self.able_after[i][t] < least - n) {
                    return;
                }
                if !self.groups.is_empty() {
                    let has = |id| self.fixed.iter().any(|p| p.0 == id) || self.chosen.iter().any(|p| self.blockers[p.0].id == id);
                    for g in self.groups {
                        if !g.subjects.iter().any(|&id| has(id)) { continue; }
                        let n = g.count(has);
                        let more = g.members.iter().filter(|&&id| !has(id) && self.blockers[i..].iter().any(|b| b.id == id)).count();
                        if n > g.at_most || n + count_u32(more) < g.at_least { return; }
                    }
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
                self.pick(i, 0, self.blockers[i].room, true, obeyed);
            }

            /// Blocker `i`'s blocks: each of its options from `from` on, with up to `left` more after it, then no more (with
            /// `first`, no block at all) - each set of attackers it can block once, in the order of its options.
            fn pick(&mut self, i: usize, from: usize, left: u32, first: bool, obeyed: u32) {
                #[cfg(test)] MULTI_SEARCH_NODES.with(|nodes| nodes.set(nodes.get() + 1));
                let blockers = self.blockers;
                let blocker = &blockers[i];
                for k in from..blocker.options.len() {
                    let (t, gain) = blocker.options[k];
                    if self.counts[t] >= self.limits[t].1 {
                        continue;
                    }
                    // Not a second member of a band it blocks already (CR 702.22h).
                    if self.bands[t] != 0 && self.chosen.iter().rev().take_while(|c| c.0 == i).any(|c| self.bands[c.1] == self.bands[t]) {
                        continue;
                    }
                    let mut more = gain + if first { blocker.any } else { 0 };
                    for &r in &self.through[t] {
                        if self.met[r] == 0 {
                            more += 1;
                        }
                        self.met[r] += 1;
                    }
                    self.counts[t] += 1;
                    self.chosen.push((i, t));
                    if left > 1 {
                        self.pick(i, k + 1, left - 1, false, obeyed + more);
                    } else {
                        self.go(i + 1, obeyed + more);
                    }
                    self.chosen.pop();
                    self.counts[t] -= 1;
                    for &r in &self.through[t] {
                        self.met[r] -= 1;
                    }
                    if self.done() {
                        return;
                    }
                }
                self.go(i + 1, obeyed);
            }
        }
        let mut search = Search {
            blockers: &blockers,
            groups: &self.groups,
            fixed,
            limits: &limits,
            best_after: &best_after,
            able_after: &able_after,
            by_target: &by_target,
            through: &through,
            meets: &meets,
            bands: &bands,
            enough: enough.map(|n| n.saturating_sub(fixed_gain)),
            counts,
            met,
            chosen: Vec::new(),
            best: None,
            ceiling: 0,
        };
        search.ceiling = search.bound(0);
        search.go(0, 0);
        let (obeyed, chosen) = search.best?;
        let mut declaration = fixed.to_vec();
        declaration.extend(chosen.into_iter().map(|(i, t)| (blockers[i].id, targets[t])));
        Some((obeyed + fixed_gain, declaration))
    }


}

/// A creature `solve` may have block, with what its blocks could obey.
struct Blocker {
    id: ObjectId,
    /// The targets it may block, each with how many requirements to block that attacker the block obeys, the most first.
    options: Vec<(usize, u32)>,
    /// How many more attackers it can block.
    room: u32,
    /// How many requirements to block whichever attacker its first block obeys.
    any: u32,
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

    const C: ObjectId = ObjectId(3);

    /// A creature that can block any number of creatures and blocks each attacking creature if able (Blaze of Glory: one
    /// requirement for each attacker, CR 509.1c) must block every attacker it may; one short obeys one fewer and is refused.
    #[test]
    fn a_creature_that_blocks_each_attacker_blocks_them_all() {
        let allowed = [(X, A), (X, B), (X, C)];
        let mut r = BlockRequirements { required: vec![lure(X, A), lure(X, B), lure(X, C)], capacities: vec![(X, u32::MAX)], ..Default::default() };
        (r.most, r.suggested) = r.solve(&allowed, &[], None, true).expect("a declaration");
        assert_eq!(r.most, 3);
        assert_eq!(r.suggested.len(), 3);
        assert!(r.judge(&allowed, &[(X, A), (X, B), (X, C)], false).complete);
        let two = r.judge(&allowed, &[(X, A), (X, C)], true);
        assert!(!two.complete && two.issue.is_some_and(|i| i.rule == "509.1c"));
        assert_eq!(two.addable, vec![(X, B)]);
        // The same attacker twice is no second block.
        let twice = r.judge(&allowed, &[(X, A), (X, A)], false);
        assert_eq!(twice.issue.map(|i| (i.creature, i.words, i.other)), Some((X, "already blocks".to_string(), Some(A))));
    }

    /// One additional creature makes two: a third block is refused, and of three attackers it must block, two are the most
    /// it can obey.
    #[test]
    fn an_additional_creature_makes_two() {
        let allowed = [(X, A), (X, B), (X, C)];
        let mut r = BlockRequirements { required: vec![lure(X, A), lure(X, B), lure(X, C)], capacities: vec![(X, 2)], ..Default::default() };
        r.most = r.solve(&allowed, &[], None, true).map_or(0, |s| s.0);
        assert_eq!(r.most, 2);
        assert!(r.judge(&allowed, &[(X, B), (X, C)], false).complete);
        let three = r.judge(&allowed, &[(X, A), (X, B), (X, C)], true);
        assert_eq!(three.issue.map(|i| (i.creature, i.words, i.rule)), Some((X, "can block only two creatures".to_string(), "509.1a")));
        assert!(r.judge(&allowed, &[(X, A)], true).addable == vec![(X, B), (X, C)]);
        // A creature that can block one is refused a second, as ever.
        let plain = BlockRequirements::default();
        let issue = plain.judge(&allowed, &[(X, A), (X, B)], false).issue;
        assert_eq!(issue.map(|i| i.words), Some("can block only one creature".to_string()));
    }

    /// "Blocks this turn if able" on a creature that can block any number is one requirement, obeyed by its first block:
    /// blocking one attacker or both obeys it alike.
    #[test]
    fn blocking_if_able_counts_once_however_many_it_blocks() {
        let allowed = [(X, A), (X, B)];
        let mut r = BlockRequirements { required: vec![blocks(X)], capacities: vec![(X, u32::MAX)], ..Default::default() };
        r.most = r.solve(&allowed, &[], None, true).map_or(0, |s| s.0);
        assert_eq!(r.most, 1);
        assert!(r.judge(&allowed, &[(X, A)], false).complete && r.judge(&allowed, &[(X, A), (X, B)], false).complete);
        assert!(!r.judge(&allowed, &[], false).complete);
    }

    /// Every way of blocking, over small random boards (allowed blocks, fewest and most blockers, bands, costs, creatures that
    /// can block two or any number, and requirements of every kind, some on the same creature twice): `solve`'s most is the most any free legal declaration
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
        for _ in 0..1000 {
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
            // Most creatures block one attacker; some two (one additional), some any number.
            let capacities: Vec<(ObjectId, u32)> = blockers
                .iter()
                .filter_map(|&b| match next(5) {
                    0 => Some((b, 2)),
                    1 => Some((b, u32::MAX)),
                    _ => None,
                })
                .collect();
            let mut r = BlockRequirements { required, bands, limits, costs, capacities, ..Default::default() };
            // Every declaration: each blocker blocks a set of the attackers it is allowed to, as many as it can at most.
            let mut all: Vec<Vec<(ObjectId, ObjectId)>> = vec![Vec::new()];
            for &b in &blockers {
                let options: Vec<(ObjectId, ObjectId)> = allowed.iter().copied().filter(|p| p.0 == b).collect();
                let mut more = Vec::new();
                for d in &all {
                    for set in 0u32..(1 << options.len()) {
                        if set.count_ones() > r.capacity_of(b) {
                            continue;
                        }
                        let mut e = d.clone();
                        e.extend((0..options.len()).filter(|&k| set & (1 << k) != 0).map(|k| options[k]));
                        more.push(e);
                    }
                }
                all = more;
            }
            let plain = r.clone();
            // Each attacker within its limits, and no creature blocking two members of one band (it blocks both through
            // either, CR 702.22h).
            let fits = |d: &[(ObjectId, ObjectId)]| {
                attackers.iter().all(|&a| {
                    let n = count_u32(d.iter().filter(|x| x.1 == a).count());
                    let (least, most) = plain.limits_of(a);
                    n == 0 || (least <= n && n <= most)
                }) && d.iter().enumerate().all(|(i, x)| !d[..i].iter().any(|y| y.0 == x.0 && plain.blocks_through(y.1, x.1)))
            };
            let most = all.iter().filter(|d| fits(d) && d.iter().all(|&(b, a)| plain.free(b, a))).map(|d| plain.obeyed(d)).max().unwrap_or(0);
            r.most = r.solve(&allowed, &[], None, true).map_or(0, |s| s.0);
            assert_eq!(r.most, most, "{r:?} over {allowed:?}");
            let legal = |d: &[(ObjectId, ObjectId)]| fits(d) && plain.obeyed(d) >= most;
            let legal_ones: Vec<&Vec<(ObjectId, ObjectId)>> = all.iter().filter(|l| legal(l)).collect();
            for d in all.iter().filter(|d| d.len() <= 2) {
                let verdict = r.judge(&allowed, d, true);
                assert_eq!(verdict.complete, legal(d), "{d:?} under {r:?}");
                let wanted: Vec<(ObjectId, ObjectId)> = allowed
                    .iter()
                    .copied()
                    .filter(|&(b, a)| !d.contains(&(b, a)) && legal_ones.iter().any(|l| l.contains(&(b, a)) && d.iter().all(|x| l.contains(x))))
                    .collect();
                assert_eq!(verdict.addable, wanted, "addable to {d:?} under {r:?} over {allowed:?}");
            }
        }
    }
}
