# Reviewing a card

Before a card lands, someone other than its author - a person, or a separate agent that had no part in writing it -
reviews it against `SKILL.md` beside this file: its non-negotiables, its "Where code goes" with the **"never" list**,
and "Scenarios: what a card owes". Read those, then review with the Oracle text, the rulings and the CR open
(`python tools\maintenance\scryfall.py "Card Name"`, `python tools\maintenance\cr.py <rule>`). A card that arrives as a
pull request is brought to this review, fixed and landed by the `review-card-pr` skill.

Log it on the card's issue: `python tools\maintenance\card-issues.py log <N> review-started` first, and at the end
`log <N> review-passed`, or `log <N> review-blocked --findings <how many blocking>` - fixed comments, nothing to write.

**The primary concerns are the code's correctness and safety and the card's adherence to the rules**; the tests are
checked for sufficiency, not exhaustiveness:

- every sentence of the Oracle text against the code, and every line of the code against the text: nothing missing,
  nothing extra, nothing approximate, nothing deviating from the rules;
- the content JSON against Scryfall's current card;
- every `CR` citation in the card's file and scenarios: `cr.py check --show <files>` prints each beside the rule it
  names (and flags a number the current rules don't have) - each rule says what the comment says it does - citations copied from
  older code can name a rule that was renumbered (CR 712.4 is meld cards, not modal double-faced cards);
- the "never" list, line by line - `card-lint.py "Card Name"` finds its mechanical items (each NEVER blocks; each LOOK
  is read, and the author's answer to it checked), the rest is read;
- number widths are decided: `docs/rules-exceptions.md` RX-5 keeps today's widths for multikicker, `ManaTally` and player
  counters (poison, energy, experience, toxic) and their existing clamps, so a u8/u16 limit on those is not a finding;
  power, toughness, counters, damage, X and mana costs are 32-bit with plain arithmetic, so a card that narrows one of
  them (`as u16`, `as i16`, a clamp or saturating arithmetic on it) or wraps it is a finding;
- no rules property set by hand (`SKILL.md`, "A card never tells the engine what the rules already decide"): a flag the
  card sets so the engine applies a rule correctly is a core gap, reported, not a card finding to patch;
- no parallel system (`CLAUDE.md`, "Use the systems that exist"): every new store, cache, flag, record, lookup or helper
  is checked against what the engine already has for that job - the event history for the past, the layers for
  characteristics, the journal for undoable state, the shared mechanics and `cantrip_state` helpers - and a second
  system for the same kind of thing blocks;
- the test points (`new-card.py "Card Name" --no-write` prints them): the author's report pairs each with the
  scenario line that catches it - check each pair, and look past the list for anything the text decides that it missed;
- the scenarios: every line covered (the coverage report), each `[Expect]` correct by the rules and able to fail - a
  limit the card's own code enforces met exactly at the limit, and both players' objects where the text reads "each",
  "target" or "you control". More tests are asked for only when the scenarios are obviously insufficient (code no
  expectation would catch being wrong), when the card is complex enough to need a specific test, or when an
  interaction has a clear potential to go wrong. A list of corner cases is not a finding;
- any core or mechanics change against its rule, its second printed card (a non-keyword recipe) and the benchmark (a
  core change);
- overlap: code in the card that an existing shared function already does, or that does the same as code in another
  card and has an engineering reason to be shared (`SKILL.md`, "Share what is the same": a rules subtlety, how a fact
  is represented, real logic). This is a finding the author fixes in this change, never an optional note and never a
  cleanup issue: grep `cantrip-mechanics` for the effect, and the other cards for the same body. A couple of lines
  that only combine pieces already shared, repeated in another card, are not a finding - and a new shared function
  that is only such glue is one;
- the client: everything the card puts into the game is shown, every question it asks has a true caption, every action
  it offers is clickable with its cost, and a browser scenario proves each new user interaction (`client.md`) - or the
  card needs nothing the client did not already do, and no browser scenario is owed. The tactical bot is never checked.

A reviewer runs only the card's scenarios and coverage (`corpus -- --card "Name"`, `python tools/maintenance/card-coverage.py "Name"`), and those of a
card whose shared code the change touched - nothing wider, except that a card adding or granting a static ability also
runs the layer contract check (`cargo test --profile checked -p cantrip-script --test layer_pass_cases`; an effect that
changes no characteristic by design, such as a granted attack cap, is listed in its `INERT` table with the reason). It **breaks the card** to see which expectation catches each
decision its code makes (a number, a filter, whose objects, a condition): `card-mutate.py "Name"` makes the mechanical
breaks and lists the survivors (it restores the file itself), and the breaks of meaning it cannot make are one
temporary edit at a time in the card's own worktree, run, then restored at once with `git checkout -- <file>` (every run goes through `cargo run`, never a prebuilt `target/.../corpus` binary: after a restore that binary still holds the break, and a review once reported an engine bug that was only a stale build); a scenario it proposes is written to its scratchpad
and tried there the same way (`corpus -- --file <path>`). It commits only the fixes the next paragraph but one gives it, cleans nothing, never
clones the repository elsewhere, and leaves `git status` clean. It reaches that worktree only by the token in its prompt (`tools\worktree.cmd join`, its first
command, and `leave` its last), never by a path it was given. Every command runs in the foreground with a long timeout
(1800000 ms, the ceiling `BASH_MAX_TIMEOUT_MS` allows: a run that outlasts its timeout is moved to the background), never in the background: a reviewer that stops to
wait on one is made to leave its worktree with its break edit still in place.

Self-reported "exact" is not proof. A finding **blocks** the card when it is a rules deviation, a code defect (an
overflow included), an item on the "never" list, overlap, or a test that is obviously insufficient - and a decision of
the card's own code (a number, a limit, a narrowing, whose objects) that no expectation would catch being wrong is
obviously insufficient, however small. The rest (wording, optional sharing) does not hold the card.

Who fixes a finding depends on what it changes. **The reviewer fixes, itself, what changes no code**: a missing or weak
expectation or scenario, a wrong citation, a stale comment, a missing `Closes #N`, and folding the card's fix-up commits
into one commit per card. It has already written and broken the probe; handing that to another agent only reloads the
card. Each expectation it adds is break-tested (it fails under the break and passes as written), and its fixes are folded
into the commit they belong to, the author kept (`git commit --fixup` and `GIT_SEQUENCE_EDITOR=true git rebase
--autosquash origin/main`); it logs `fixes-committed --commit <sha>` (refused without the sha) and then `review-passed`. **A finding that changes code** - the
card's logic, shared code, the core - goes back to an author, so that what the fix decides is reviewed by someone who
did not write it: the reviewer reports it, logs `review-blocked`, and confirms the fix when it comes back. The
reviewer's verdict is final: the coordinator lands on it and does not review the card again. Nothing is changed
silently: every fix is in the report.

**A reviewer never accepts a rules deviation, or anything that risks one** (`CLAUDE.md`, "No rules deviations"): a
clamp, a floor, a fixed-size list that cuts down or drops, an approximation that holds only for today's cards, a path
left untested "until a card needs it". Each blocks. "Record it as a known issue" is never an answer to a rules finding.
A suspected rules bug elsewhere is reported as a finding to be fixed, not filed away; a path no scenario can reach gets
a Rust test with test-only cards. Only a human may allow a deviation, in `docs/rules-exceptions.md`.
A rules finding needs a card that reaches it: any card legal in a format Cantrip supports, implemented or not, or any
card the engine implements, legal or not (`CLAUDE.md`, "Bug priority"). Name the cards; a case no such card can reach
is not raised.

Report briefly: PASS (with the commits the card now stands at, ready to land, and each fix the reviewer made, with the
break that proves it), or the blocking findings an author must fix - each with file:line, what is wrong and the rule -
then any non-blocking notes apart.
