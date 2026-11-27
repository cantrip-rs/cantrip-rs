# Many cards: the coordinator and its agents

The coordinator dispatches, lands and keeps the queue; it does not implement cards itself. Card agents run on Sonnet;
core-feature agents and review agents run on Opus. Agents read only what their work needs: a card agent `SKILL.md`
(and the `write-game-script` skill), a reviewer `review.md` and the parts of `SKILL.md` it names, a core-feature agent
`SKILL.md`'s "Where code goes" and the rule it builds.

0. **The worktree pool is the tool's: `tools\worktree.cmd`** (`tools/maintenance/worktrees.py`, its docstring the
   whole model). Work happens in a pool of worktrees, `.claude/worktrees/pool-<n>`, each with its own warm `target/`;
   their number (about 4) is also the cap on build-heavy agents at once, because each cargo build takes every core.
   Nothing reaches a slot but through the tool, and nobody picks a slot or passes its path. The pool is opt-in per
   machine (its hooks in the local settings): when `acquire` says they are not installed, stop and offer the user the
   `worktree-pool-setup` skill.
   - **New work is a new lease**: `worktree.cmd acquire --for "card: <Name>"` (or `"gap: <rule>"`, `"PR #N"`) gives a
     clean slot on today's `origin/main`, its path and a lease ID that stay with you - never in a prompt or a message.
     `NO WORKTREE AVAILABLE` means wait for a release; growing the pool (`create`) is the user's call. A lease is
     never reused for the next card.
   - **Every agent in the slot is let in by a token**: `worktree.cmd token <lease>` (a reviewer's too: it breaks the card in place and restores it, `review.md`) -
     single-use, dead in two minutes - pasted into its prompt as the token's output says; the agent's first command is
     `join`. That includes an agent resumed with SendMessage to work in the slot again: the message carries a fresh
     token, since an agent that stopped has left.
   - **One member at a time**: the next token is made only once the member has left (`leave`, or its agent stopping -
     the SubagentStop hook makes it leave). A long-running agent keeps its slot until then; nothing is ever started
     beside it. `evict` only a member whose agent you know has ended.
   - **Release when it has landed**: `worktree.cmd release <lease>` - refused while a member is in, while the slot is
     dirty or while its commits' patches are not on `origin/main`; `--discard --reason` only for work truly abandoned.
   - **`worktree.cmd status`** lists every lease, its purpose and member - read it rather than your memory of the pool
     (after a compaction, before every `acquire`). `reconcile` finds what is stale. A slot set aside as needing
     attention is the user's: `status pool-<n>` shows what it holds, and `clean pool-<n> --reason` empties it on
     their say.

   The hooks deny an edit or shell command into a slot by anyone but its member, and the main thread while a member
   is in it; and they keep a member in its own tree: its edit outside the tree (the temp directory aside), or its
   shell command run from or naming the main checkout, is denied. When an agent's work truly needs a place outside,
   it says so and the lease holder grants that path: `worktree.cmd allow <lease> <path>`, good until the release. Agents are started without the Agent tool's own worktree isolation. Every build stays in the slot's own
   `target/`: never `CARGO_TARGET_DIR` or `--target-dir` pointed at another tree's, and never a copied `target/` -
   cargo's freshness records name absolute source paths, so a shared or copied `target/` compiles another tree's files
   into a test run that claims to test this one.
1. **One card per agent, one lease per card.** Before starting an agent, `card-issues.py next <format>`: an agent
   started on an empty format spends minutes finding that out. Once a day, before the first agents start, run
   `python tools/maintenance/scryfall.py --refresh` from the main checkout: only the coordinator refreshes the shared
   Scryfall cache, and agents only read it (it is refused in a pool slot). For each card, `acquire` a lease and start a card agent
   with a token for it: the agent claims the card (`claim --format <format>`, or the card named), implements it,
   commits, reports and leaves. Its review and fixes follow in the same lease, each agent with its own token (4). When
   the card has landed, `release`; the next card takes a new lease and a fresh agent: the finished card's history is no
   help to the next one and only costs tokens. Cards go in list order, never pre-sorted by difficulty. The
   coordinator's job is the format and how many leases are out, not the claims, and it releases nothing an agent has
   not claimed.
2. **A card agent's prompt** carries its token (never a path) and the format, points it at `SKILL.md` (read with the Read tool - it is
   longer than a shell result shows) and the `write-game-script` skill, and adds no requirement the skill does not
   make. It says: claim before any work; `new-card.py` first; the core is closed to it (a missing keyword module may go
   in `cantrip-mechanics`); run only its card's scenarios and coverage and those of a card whose file or shared
   mechanic it changed; one commit per card, `Closes #N`, mechanics and client commits before it; no push; every command in the
   foreground with a long timeout (1800000 ms, the ceiling `BASH_MAX_TIMEOUT_MS` allows: a run that outlasts its timeout is moved to the background), never in the
   background (an agent that stops to wait on one is made to leave its slot, and loses it). The harness moves a command
   that runs past about 180 s into the background whatever its timeout (card-mutate, a cold build, `test.cmd`): the
   agent then waits for it with short blocking foreground checks of its output, never ends its turn and never uses
   Monitor while it runs - the prompt says so, since three agents in one run lost their slot to it; source files are
   changed with the Edit and Write tools, never rewritten by a PowerShell `Get-Content`/`Set-Content`/`[IO.File]` pass
   (it re-encodes non-ASCII text such as "−" and "Dáin", and a relative `[IO.File]` path lands in the main checkout);
   every shell command `cd`s
   into its worktree first (the session's working directory is shared, and moves); scratch files (patches, probes) go
   in a scratchpad subfolder named for its agent id, never at the top, where another slot's agent can overwrite them; and a short
   report when committed - its commits, its own scenarios' pass line run on the commit as it stands (again after each
   round of fixes; a report without it is not sent to review), coverage, each test point with the line that catches it and what it
   broke, client needs, anything odd in the core, and what in `new-card.py`'s
   output it used, what was noise, what it searched for by hand and what cost it failed runs (the coordinator's
   material for step 7). **A bug that does not block its card is out of the agent's scope** - a card, core, review or
   fix agent alike: it files the bug on GitHub (`gh issue create --label bug`, the rule, the game that shows it, the
   file and line), names the issue in its report and moves on; it fixes only what its own card cannot be exact
   without. The coordinator dispatches the fix (a pri-0/1 at once, in its own lease). A card blocked by the core is never faked and never released: the agent keeps the claim,
   commits nothing the gap would change, sends a gap report - the card, its issue, the exact rule (CR), what
   `cantrip-api` lacks and what it learned - and ends; its lease is released (nothing landed is lost: the gap's
   commits, if any, are pushed to a branch first) and the next card takes a new one meanwhile. The agent logs its
   points on the card's issue (`claims.md`, "The progress log": `committed`, `fixes-committed`, `blocked`); a fresh
   agent finishing a card that waited logs `unblocked`.
3. **Gaps.** For a reported gap the coordinator `acquire`s a lease and starts a core-feature agent (Opus) with its token: the core open to it,
   the feature built to its rule for every card the rule describes, the benchmark run, its own commits
   (`Engine: ...`), no card work. Gaps several waiting cards share go to one feature agent; feature agents work on
   separate rules areas. As soon as the feature passes its review, a fresh card agent, given the gap report, finishes
   the waiting card in a new lease, cherry-picking the reviewed core commits rather than waiting for them to land. Other
   running agents are told what landed so they rebase. A pri-0/1 fix in progress pauses nothing else: card agents and
   the PR reviewer keep building, reviewing and landing, and the fix rebases over them before it lands. Only work that
   needs the fix waits for it, and the coordinator says which. Only a gap the coordinator decides not to build is released,
   with the gap recorded on the issue.
4. **Review agents** (Opus) review inside the card's lease - let in by a token once the card agent has
   left, on the commit as it stands, using its warm build - never in a worktree of their own or a clone; they break the
   card one temporary edit at a time and restore it (`review.md`), and leave `git status` clean; they log `review-started` and their verdict on the card's issue, and
   leave. Their prompt points them at
   `review.md`, the card's issue number, the commit, the card's Oracle text and the CR rules it
   touches, and at correctness, safety and rules adherence; it hands them no list of corner cases to demand tests for.
   Each card is reviewed as soon as it is committed. **The reviewer fixes what changes no code itself** (`review.md`:
   a missing expectation or scenario, a citation, a comment, a `Closes` trailer, one commit per card), break-tested and
   committed in the slot, and passes the card ready to land - no fix agent for a scenario line it has already written.
   Only a finding that changes code goes back to the card agent (SendMessage, a fresh write token in the message, once
   the reviewer has left), or to a fresh Sonnet agent when the author is gone. The message says to fold each fix into
   the commit it belongs to (`git commit --fixup` and `git rebase -i --autosquash` run as `GIT_SEQUENCE_EDITOR=: git
   rebase --autosquash -i origin/main`), so the slot still holds its Mechanics commits and one Card commit - a fix
   stacked as a new Card commit costs the reviewer a restructuring round; the reviewer confirms the fix the same
   way, with a fresh token, and fixes any test-only remainder itself. **The reviewer's verdict is final:** the
   coordinator lands on it and does not review the card a second time - it reads the report for what it must act on
   (a code finding to dispatch, a core gap, a question only the user can answer) and checks only what landing needs.
5. **Land** each reviewed card or core change on `main` (cherry-pick its commits; a conflict stops for a look), then
   one test run for everything ready to land together: `tools\test.cmd /engine` (the Rust workspace, about a minute and
   a half: the corpus with 0 fail - known bugs reported beside it - and the verify tests) when the batch touches only
   cards, content, mechanics and scenarios; the whole `tools\test.cmd` when it touches the core (`cantrip-state`,
   `cantrip-rules`, `cantrip-api`), the host, the protocol, the client or anything under `dotnet/`, and otherwise at a
   much lower cadence (the last landing of a session, or every ten or so). `git fetch` and `git rebase origin/main`
   right before every push. If more commits arrive while the run goes, rebase, build, and run only the scenarios of
   the cards and mechanics this landing touches before pushing. Push, then `card-issues.py log <N> landed --commit
   <sha>` for each card (and `review-passed` first where the coordinator verified review fixes itself); end the card's
   agents, then `worktree.cmd release <lease>`; the next card takes a new lease (1).
6. **Report** what landed, what waited on which core gap and how it was built, the corpus line, and
   `card-issues.py times` for the cards of the run.
7. **Improve the process, always.** After each card, read the agent's report and the review for friction - a search by
   hand the tools could have answered, a failed run the skills could have prevented, a finding the agent could have
   caught itself - and fix the tool or the skill before the next card. Weigh every change against three pillars in
   this order, never traded the other way: (1) the code is correct and safe, (2) the Magic rules are enforced exactly,
   (3) cards are implemented fast. Watch for steps whose cost buys little toward a validated, rules-exact card: work
   done twice, waits (an idle blocked agent, a review held for a batch, cold builds, a rebase loop against a moving
   `main`), demands beyond what a card owes, and prompts that add requirements the skill does not make. Raise a change
   with the user when it trades anything; write approved changes into these files (and `import-cards`, CLAUDE.md or
   the tools) in the same session, so the next run starts from the better process.
