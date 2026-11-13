---
name: review-card-pr
description: Review a pull request that adds or fixes a card (or the mechanics it shares) against the add-cards standard, then fix what the review finds and land it as if the card had been built here - the PR's commits in a fresh worktree-pool slot, straight to an Opus review agent, a fresh Sonnet agent for the fixes, then the coordinator's landing. Use whenever a new card implementation from someone else is reviewed - a GitHub PR, a contributor's branch, commits handed over.
---

# Reviewing a card pull request

A card that arrives as a pull request is handled exactly like a card a local card agent has just committed: the
`add-cards` skill is the standard, and its `coordinator.md` is the procedure from step 4 (review) onward. This skill
only says how the PR's commits get there. Nothing here relaxes the standard. You are the coordinator: you dispatch,
check and land; you do not review or fix the card yourself.

Take pull requests oldest first unless the user names one, **up to three in review at once**, each in its own lease:
at most three review or fix agents working at a time, since each build takes every core. Landings go one at a time.
Report each PR as it lands or stops.

## 1. The PR's commits in a fresh pool slot

Take a lease on the worktree pool (`coordinator.md` step 0): `tools\worktree.cmd acquire --for "PR #<N>"` gives a
clean slot on today's `main`, its path and the lease ID, which stay with you. No slot free: wait, or ask the user to
grow the pool. With no member in the slot yet, put the PR on it:

```
cd <the path acquire printed>
git fetch origin pull/<N>/head:pr-<N>
git cherry-pick origin/main..pr-<N>        # the PR's own commits, in order, authorship kept
```

A cherry-pick conflict is resolved here when it is mechanical: each side's intent survives untouched and the merge is
plain - independent additions side by side (imports, entries, match arms, variants, `mod` lines, tests, docs), a
rename or move on one side applied to the other's code, a version or key number both took (the next free one), or a
PR commit already on `main` (left out). Build it, tell the review agent which commits were resolved and how so it
reviews the resolution too, and name each resolution in the landing comment. A design conflict - both sides changing
the same logic differently, one reworking what the other builds on, a resolution that would choose between designs
or rewrite either side - is never resolved by guesswork: comment on the PR asking the contributor to rebase onto
`main`, naming the conflicting files and the commits on `main` they meet, and release the lease with `--discard`
(`tools\pr-watch.cmd` takes the PR up again when they push). Note the
card(s) the PR adds or changes, every other card or shared function it touches, and the card's issue (`Closes #N` in
the commits, or `gh issue list --search "<Card Name> in:title" --state all`; a card with no issue gets one with
`card-issues.py claim --pr "Card Name"` (the PR's own branch has the card, so `--pr`) only if the user agrees - the claim may be the contributor's).

## 1a. A batched PR gets a dedupe pass first

A PR of more than about ten cards is reviewed in batches of about ten, one Opus reviewer per batch. Before the first
batch, one fresh Sonnet agent (a card agent's prompt, as in step 3) makes a single pass over **every** card in the PR
and fixes the overlap a reviewer would otherwise block each batch on:
- `card-dupes.py` and `card-lint.py` on each card;
- a hand-written copy of a shared function, target constant, trigger constant or keyword (seek out, cycling, equip,
  enters tapped, drains, legal targets, cost reductions, "opponents attacked this turn", token creation);
- "<subtype>s you control" filtered to creatures (CR 109.2);
- a loop over players for one simultaneous action;
- a clamped or saturated count.

Each fix is a `fixup!` of the commit it belongs to; a shared function that does not yet exist is its own `Mechanics:`
commit before the cards that use it. It never wraps a single existing shared call in a new function (thin glue, such as
an effect that only calls `targets_fight(g, item, 0, 1)`): sharing is for real rules or game-state work. The agent runs the scenarios of the cards it changed and reports what it changed,
card by card. The batch reviewers then start on the deduped commits.

On PR #2855 (197 cards) about 85% of batches came back blocked, mostly on this overlap, and each blocked batch cost a
fix round and a re-review. One pass up front turns most batches into a single review.

## 2. Straight to the review: an Opus agent

Start a fresh agent (`model: "opus"`, no worktree isolation) exactly as `coordinator.md` step 4 says for a review
agent: let into the lease by a token (`worktree.cmd token <lease>`, the token in its prompt, never the path; not
`--read-only`, since `review.md` has it break the card one temporary edit at a time and restore it), on the commits as they stand, pointed at `review.md`, the card's issue number,
the commits, the card's Oracle text and the CR rules it touches - and nothing else, no opinion of the card and no list
of corner cases. Add only what a PR makes different:

- the commits are someone else's; review **every** file the PR changes, including any other card a refactor moved onto
  a shared function, since that card's behaviour changes too, and run those cards' scenarios as well;
- the seams are checked line by line: the card file holds only that card's own conditions, values, branching and
  sequencing, and every rules concept it touches (a name and name identity, mana value, a zone destination, a choice,
  "each player", the past, a keyword, a token) goes through the verb, query or shared mechanic that already owns it -
  grep `crates/cantrip-api` and `crates/cantrip-mechanics` for the concept, not for the name the author would have
  given it; a new or widened shared function names a second printed card that uses it exactly as written, is the same
  for every caller, and does real rules or game-state work rather than glue over a call that is already shared.

The reviewer also fixes, itself, what changes no code (`review.md`): a missing expectation, a citation or comment, the
`Closes #N` trailer, and folding the PR's fix-up commits so each card lands as one commit, the contributor's authorship
kept. Its verdict is final; the coordinator does not review the PR a second time.

## 3. Fixes that change code: a fresh Sonnet agent

With a finding that changes code (the card's logic, shared code, the core), start a fresh agent
(`model: "sonnet"`, no worktree isolation) in the same lease, with a write token once the reviewer has left, as the
card agent that would have written the card: its
prompt is a card agent's (`coordinator.md` step 2 - `SKILL.md` read with the Read tool, the `write-game-script` skill,
the core closed to it, only the touched cards' scenarios and coverage, no push), plus the findings verbatim and that
it fixes them on top of the PR's commits, logs `fixes-committed` on the issue and reports how each was fixed. A fix
that needs the core is a gap report, handled as `coordinator.md` step 3 says.

Send the fix commits back to the review agent (SendMessage, its context kept, a fresh token in the message
once the fix agent has left) to confirm each finding is resolved and
nothing new is broken; loop until it passes, or stop and tell the user when a finding needs a decision only they can
make. No blocking findings: nothing to fix.

A re-review is scoped to what the fixes changed; the landing's test run (step 4) is the full gate. It builds the head (never
each commit of the stack: no `git rebase -x "cargo check ..."`, here or at the landing), runs
the scenarios and coverage of the cards whose files changed and of every card that calls changed shared code, and
breaks the changed lines (shared code included) to check a scenario catches each break. It does not run the whole
`cargo test` or the other cards' scenarios again, and it does not rebase
the stack onto a newer `main` (the landing does that). Tell it this in the SendMessage.

## 4. Land by merging the PR itself

A PR lands **merged**, never closed: the reviewed and fixed commits go onto the PR's own branch, and `main` is
fast-forwarded to that branch, so GitHub marks the PR merged with exactly the commits that were tested.

- **A card that needs a rewrite from scratch** (the review finds its file unusable as a base, not a set of fixes) is
  dropped from the PR, not rewritten in it: remove that card's commits, leave its issue open with a comment saying why,
  and merge the rest. A PR whose every card needs that is not landed (below).
- **Land** as `coordinator.md` step 5 says, but on the PR's branch:
  1. Check that maintainers may push to it: `gh pr view <N> --json maintainerCanModify,headRepositoryOwner,headRefName`.
     If `maintainerCanModify` is false, comment on the PR asking the contributor to allow edits by maintainers, and
     wait; never close the PR to land it.
  2. In the main checkout, put the slot's final commits on `main` (`git fetch`, `git rebase origin/main`, cherry-pick
     `origin/main..<slot branch>`), and run the test run step 5 names on exactly that head.
  3. If it passes, `git fetch` again. If `origin/main` moved, rebase, and run only the touched cards' scenarios (the
     whole run again if the core moved).
  4. Force-push that head to the PR's branch:
     `git push --force-with-lease https://github.com/<owner>/<repo>.git HEAD:<headRefName>`.
  5. Then fast-forward `main` to the same commit: `git push origin HEAD:main`. GitHub marks the PR merged by itself,
     because its head is now on `main`.
  6. Check with `gh pr view <N> --json state` that it says `MERGED`. Never `gh pr close`.
- **Afterwards:** `card-issues.py log <N> landed --commit <sha>` for each card, and a `gh pr comment <N>` with the
  commit range, what the review found and what was changed (and any card dropped, with why). Delete `pr-<N>`, then
  `tools\worktree.cmd release <lease>`; the next PR takes a new lease.
- **A PR that does not land at all:** comment on it with why, leave it open for the contributor, and `release --discard
  --reason "PR #<N> not landed: ..."`, since its commits stay on the PR.

## 5. Report

Per PR, to the user: the card(s), the review's blocking and non-blocking findings (and any dropped, with why), what
the fix agent changed, the coverage and corpus lines, and what landed with its hashes - or why it did not land.
