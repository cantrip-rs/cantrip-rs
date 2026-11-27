"""GitHub issues for the cards a format plays, and who is working on which.

    python tools/maintenance/card-issues.py sync <format> [--dry-run] [--limit N]
    python tools/maintenance/card-issues.py next <format> [-n 5] [--json]
    python tools/maintenance/card-issues.py claim "Card Name" ["Other Card" ...]
    python tools/maintenance/card-issues.py claim --format <format> [-n 1]
    python tools/maintenance/card-issues.py release "Card Name" ["Other Card" ...]
    python tools/maintenance/card-issues.py status
    python tools/maintenance/card-issues.py log <N> <event> [--commit SHA | --findings N | --rule CR]
    python tools/maintenance/card-issues.py times [--days 30]

The cards a format plays are docs/formats/<format>.json, best-played first (format-scrape.py makes it; for
Commander, edhrec-scrape.py). Each card that has no card file yet is one issue, "Card: <Name>", labelled `card`,
`format:<format>` (`Commander` for Commander; a card several formats play has several) and a `priority:` label for how much of the format plays it. `sync` makes the issues that are
missing, labels the ones that exist, and closes those whose card now has a file. Run it again after a new scrape.

A claim is a comment on the issue ("Claimed by @you"), because the people who work on cards cannot all be assigned
to or label an issue. A claim holds for 24 hours from the last sign of work, and then the card is free for anyone:

    - a renewal (the claim command again, by the holder);
    - a commit on an open pull request that closes the issue ("Closes #N" in its description) or is from a branch
      named card/<slug>; a pull request itself is a claim by its author;
    - a push to a branch named card/<slug> in this repository, for work with no pull request.

The earliest live claim wins: `claim` posts its comment, reads the issue again, and says so when someone else's claim
came first. A claimant is a GitHub account and a checkout: the comment names the checkout (`--as`, else
$CANTRIP_CLAIMANT, else the checkout's directory name), so two agents working under one account in their own
worktrees hold cards apart and never take the same one. Pushes and pull requests carry only the account, and renew
the live claim of any checkout of that account. `next` lists what can be taken - issues nobody holds live - in the format's order; `claim` on a card
with no issue opens the issue first, then claims it. Commit messages and pull request descriptions of a card say
`Closes #N`, so the issue closes when it lands.

The repository is origin's (--repo owner/name to override); gh is the GitHub client and must be signed in.
"""

import argparse
import concurrent.futures
import datetime
import functools
import importlib.util
import json
import os
import re
import subprocess
import sys
import time

sys.stdout.reconfigure(encoding="utf-8")
sys.stderr.reconfigure(encoding="utf-8")
sys.dont_write_bytecode = True  # no __pycache__ left in tools/maintenance

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(os.path.dirname(HERE))
FORMATS = os.path.join(ROOT, "docs", "formats")
HOLD = datetime.timedelta(hours=24)
BATCH = 1  # cards claimed at once by default: one, since a card takes minutes and a claim keeps others off it
CLAIM = "<!-- cantrip-claim -->"
CLAIMANT = "<!-- cantrip-claimant: {} -->"
CLAIMANT_RE = re.compile(r"<!-- cantrip-claimant: (\S+) -->")
RELEASE = "<!-- cantrip-release -->"
CARD = "<!-- cantrip-card: {} -->"
# The progress log: fixed comments an agent posts as a card passes each point (`log`), read back by `times`. Each is
# {event: (text, the one input it takes or None)}; the comment's own time is the event's.
PROGRESS = "<!-- cantrip-progress: {} -->"
PROGRESS_RE = re.compile(r"<!-- cantrip-progress: ([\w-]+) -->")
EVENTS = {
    "committed": ("Implementation committed in {commit}; waiting for review.", "commit"),
    "review-started": ("Review started.", None),
    "review-passed": ("Review passed.", None),
    "review-blocked": ("Review found {findings} blocking finding(s); back to the author.", "findings"),
    "fixes-committed": ("Review fixes committed in {commit}.", "commit"),
    "blocked": ("Waiting on an engine change (CR {rule}).", "rule"),
    "unblocked": ("Engine change ready; implementation resumed.", None),
    "landed": ("Landed on main in {commit}.", "commit"),
}
# A card's priority label: the first tier whose score it reaches. The score is the percent of the format's decks
# that play the card (format-scrape.py).
TIERS = [(10.0, "priority:high"), (3.0, "priority:medium"), (0.0, "priority:low")]
LABELS = {
    "card": ("b4a8d1", "A card the engine does not have yet"),
    "priority:high": ("d73a4a", "Played in a tenth or more of the format's decks"),
    "priority:medium": ("fbca04", "Played in a few of the format's decks"),
    "priority:low": ("c5def5", "Played in some of the format's decks"),
}
FORMAT_COLOR = "1d76db"
# A format's label is format:<format> unless it is named here.
FORMAT_LABELS = {"commander": "Commander"}


def format_label(format_):
    return FORMAT_LABELS.get(format_, f"format:{format_}")


def load(name):
    """A sibling script (their names have hyphens) as a module."""
    spec = importlib.util.spec_from_file_location(name.replace("-", "_"), os.path.join(HERE, name + ".py"))
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def repository(override):
    if override:
        return override
    url = subprocess.run(["git", "-C", ROOT, "remote", "get-url", "origin"], capture_output=True, text=True,
                         check=True).stdout.strip()
    match = re.search(r"github\.com[:/]([^/]+/[^/]+?)(?:\.git)?$", url)
    if not match:
        sys.exit(f"origin is not a GitHub repository: {url}")
    return match.group(1)


def gh(*args, body=None):
    """One gh call: its output, parsed as JSON when there is any. Waits out GitHub's rate limits."""
    for attempt in range(6):
        run = subprocess.run(["gh", *args], input=None if body is None else json.dumps(body), capture_output=True,
                             text=True, encoding="utf-8")
        if run.returncode == 0:
            return json.loads(run.stdout) if run.stdout.strip() else None
        if "rate limit" in run.stderr.lower() or "secondary" in run.stderr.lower():
            wait = 60 * (attempt + 1)
            print(f"rate limited; waiting {wait}s", file=sys.stderr)
            time.sleep(wait)
            continue
        sys.exit(f"gh {' '.join(args)}: {run.stderr.strip()}")
    sys.exit("gh: still rate limited")


def graphql(query, repo, **fields):
    """Every page of a paginated query ($endCursor), as a list of pages."""
    owner, name = repo.split("/")
    extra = [x for k, v in fields.items() for x in ("-f", f"{k}={v}")]
    pages = gh("api", "graphql", "--paginate", "--slurp", "-f", f"query={query}", "-F", f"owner={owner}",
               "-F", f"name={name}", *extra)
    return pages if isinstance(pages, list) else [pages]


def when(text):
    return datetime.datetime.fromisoformat(text.replace("Z", "+00:00"))


# Open issues with their claim comments; closed ones (a card done, or dropped) need no comments, which are most of
# what a page of issues costs. Every issue, not only those labelled `card`: a contributor who cannot label opens a
# card's issue with no label, and it is known by its title. Missing one here would open a duplicate and miss its claim.
ISSUES = """query($owner:String!,$name:String!,$endCursor:String){repository(owner:$owner,name:$name){
issues(first:100,after:$endCursor,states:OPEN){pageInfo{hasNextPage endCursor}nodes{number title state
labels(first:20){nodes{name}}comments(last:20){nodes{author{login}body createdAt}}}}}}"""
CLOSED = """query($owner:String!,$name:String!,$endCursor:String){repository(owner:$owner,name:$name){
issues(first:100,after:$endCursor,states:CLOSED){pageInfo{hasNextPage endCursor}nodes{number title
state labels(first:20){nodes{name}}}}}}"""
ONE_ISSUE = """query($owner:String!,$name:String!,$number:Int!){repository(owner:$owner,name:$name){
issue(number:$number){number title state labels(first:20){nodes{name}}comments(last:50){nodes{author{login}body
createdAt}}}}}"""
# A grouped card PR may close twenty issues; retain activity for every card in the group.
PULLS = """query($owner:String!,$name:String!,$endCursor:String){repository(owner:$owner,name:$name){
pullRequests(first:50,after:$endCursor,states:OPEN){pageInfo{hasNextPage endCursor}nodes{number createdAt
headRefName author{login}closingIssuesReferences(first:100){nodes{number}}commits(last:30){nodes{commit{committedDate}}}}}}}"""
BRANCHES = """query($owner:String!,$name:String!,$endCursor:String){repository(owner:$owner,name:$name){
refs(refPrefix:"refs/heads/card/",first:100,after:$endCursor){pageInfo{hasNextPage endCursor}nodes{name target{
...on Commit{committedDate author{user{login}}}}}}}}"""


def title(name):
    return f"Card: {name}"


def with_events(issue):
    """An issue as GitHub gives it, its labels a set and its claim and release comments as events."""
    issue["labels"] = {label["name"] for label in issue["labels"]["nodes"]}
    issue["events"] = []
    for comment in (issue.get("comments") or {}).get("nodes", []):
        user = (comment["author"] or {}).get("login")
        kind = "claim" if CLAIM in comment["body"] else "release" if RELEASE in comment["body"] else None
        if kind and user:
            tag = CLAIMANT_RE.search(comment["body"])
            issue["events"].append((when(comment["createdAt"]), kind, identity(user, tag and tag.group(1))))
    issue["from_comments"] = len(issue["events"])  # pull request and branch events come after
    return issue


_state = {}


def read_state(repo, fresh=False):
    """The issues about cards, with what is known of who holds each: {number: issue}, and each issue's events. Read
    once a run (`fresh` reads again); its four queries go to GitHub at once, each taking seconds."""
    if repo in _state and not fresh:
        return _state[repo]
    with concurrent.futures.ThreadPoolExecutor(4) as pool:
        opened, closed, pulls, branches = pool.map(lambda q: graphql(q, repo), (ISSUES, CLOSED, PULLS, BRANCHES))
    issues = {}
    for page in opened + closed:
        for issue in page["data"]["repository"]["issues"]["nodes"]:
            issues[issue["number"]] = with_events(issue)
    by_slug = {slug_of(i["title"]): i for i in issues.values() if i["title"].startswith("Card: ")}
    for page in pulls:
        for pull in page["data"]["repository"]["pullRequests"]["nodes"]:
            user = (pull["author"] or {}).get("login")
            linked = [n["number"] for n in pull["closingIssuesReferences"]["nodes"]]
            if pull["headRefName"].startswith("card/") and pull["headRefName"][5:] in by_slug:
                linked.append(by_slug[pull["headRefName"][5:]]["number"])
            for number in set(linked) & set(issues):
                issues[number]["events"].append((when(pull["createdAt"]), "claim", user))
                for node in pull["commits"]["nodes"]:
                    issues[number]["events"].append((when(node["commit"]["committedDate"]), "ping", user))
    for page in branches:
        for ref in page["data"]["repository"]["refs"]["nodes"]:
            issue = by_slug.get(ref["name"])
            if issue and ref["target"]:
                user = ((ref["target"].get("author") or {}).get("user") or {}).get("login")
                issue["events"].append((when(ref["target"]["committedDate"]), "ping", user))
    _state[repo] = issues
    return issues


def read_issue(repo, issue):
    """One issue's claim and release comments read again (after claiming it, to see whose claim came first), its
    pull request and branch events as they were."""
    owner, name = repo.split("/")
    data = gh("api", "graphql", "-f", f"query={ONE_ISSUE}", "-F", f"owner={owner}", "-F", f"name={name}",
              "-F", f"number={issue['number']}")
    again = with_events(data["data"]["repository"]["issue"])
    again["events"] += issue["events"][issue["from_comments"]:]
    return again


def slug_of(issue_title):
    return slug(issue_title[len("Card: "):])


def identity(login, tag):
    """A claimant: the account, and the checkout it claimed from when the claim names one."""
    return f"{login}/{tag}" if tag else login


def login_of(who):
    return who.split("/")[0] if who else who


def named(who):
    login, _, tag = who.partition("/")
    return f"@{login}" + (f" ({tag})" if tag else "")


def holder(issue, now=None):
    """(claimant, since, last) of the live claim on an issue, or None."""
    now = now or datetime.datetime.now(datetime.timezone.utc)
    user = since = last = None
    for time_, kind, who in sorted(issue["events"], key=lambda e: (e[0], e[1] != "claim")):
        if kind == "claim":
            if user is None or who == user or time_ - last >= HOLD:
                user, since, last = who, (since if who == user else time_), time_
        elif kind == "ping":
            if user is None and who:
                user, since, last = who, time_, time_
            elif who is None or who == login_of(user):
                last = max(last, time_)
        elif kind == "release" and who in (user, login_of(user)):
            user = since = last = None
    if user and now - last < HOLD:
        return user, since, last
    return None


missing = load("missing-cards")
slug = missing.slug


@functools.cache
def done_anywhere():
    """Every card done in this checkout or on any local branch of the repository: a card is given out to nobody once
    its content file exists, landed or not (a reviewed card waiting on a land/ branch, another worktree's slot)."""
    done = missing.implemented()
    # One walk of every commit on a local branch that this checkout lacks, for the card files they add.
    added = subprocess.run(["git", "-C", ROOT, "log", "--branches", "--not", "HEAD", "--diff-filter=A", "--name-only",
                            "--format=", "--", "content/*/cards/*.json"], capture_output=True, text=True).stdout.split()
    done.update(slug(os.path.basename(path)[: -len(".json")]) for path in added)
    return done


def card_issue_body(name, card, entries):
    """The text of a card's issue: what the card is, who plays it, and how to take it."""
    lines = []
    if card:
        faces = card.get("card_faces") or [card]
        for face in faces:
            stats = f" {face['power']}/{face['toughness']}" if "power" in face else ""
            lines += [f"**{face['name']}** {face.get('mana_cost', '')}", f"*{face['type_line']}{stats}*"]
            lines += ["> " + line for line in face.get("oracle_text", "").splitlines()] + [""]
    for format_, entry in entries:
        if "goldfish" in entry:
            lines.append(f"- **{format_.title()}**: in {entry['score']:g}% of the decks read "
                         f"(MTGGoldfish {entry['goldfish']:g}%, MTGTop8 {entry['top8']:g}%)")
        else:  # edhrec-scrape.py
            top = f"in {entry['top']:g}% of the decks that can play it (edhrec.com/top); " if entry["top"] else ""
            lines.append(f"- **{format_.title()}**: {top}in {entry['commanders']:g}% of the decks of EDHREC's top "
                         f"commanders, on {entry['among']} of their pages")
    lines += ["", "To take it: `python tools/maintenance/card-issues.py claim \"" + name + "\"` (the add-cards skill "
              "does this itself). The claim lasts 24 hours from your last push; open a pull request with "
              "`Closes #<this issue>` and it stays yours while the pull request is worked on.", "",
              CARD.format(name)]
    return "\n".join(lines)


def ensure_labels(repo, names):
    for label in sorted(names):
        color, description = LABELS.get(label, (FORMAT_COLOR, "Cards this format plays"))
        run = subprocess.run(["gh", "api", "-X", "POST", f"repos/{repo}/labels", "-f", f"name={label}",
                              "-f", f"color={color}", "-f", f"description={description}"], capture_output=True,
                             text=True)
        if run.returncode != 0 and "already_exists" not in run.stdout + run.stderr:
            print(f"label {label}: {run.stderr.strip()}", file=sys.stderr)


def tier(score):
    return next(label for floor, label in TIERS if score >= floor)


def formats_of():
    """{format: {card name: entry}} for every docs/formats/*.json."""
    result = {}
    for entry in sorted(os.listdir(FORMATS)) if os.path.isdir(FORMATS) else []:
        if entry.endswith(".json"):
            with open(os.path.join(FORMATS, entry), encoding="utf-8") as f:
                data = json.load(f)
            result[data["format"]] = {c["name"]: dict(c, rank=i) for i, c in enumerate(data["cards"])}
    return result


def scryfall_cards(names):
    card_list = load("card-list")
    found, _ = card_list.resolve(list(names))
    return found


def labels_for(name, formats):
    """The labels a card's issue should carry, from every format that plays it."""
    entries = [(f, cards[name]) for f, cards in formats.items() if name in cards]
    labels = {"card"} | {format_label(f) for f, _ in entries}
    if entries:
        labels.add(tier(max(e["score"] for _, e in entries)))
    return labels, entries


def sync(args, repo):
    formats = formats_of()
    if args.format not in formats:
        sys.exit(f"no docs/formats/{args.format}.json; run format-scrape.py {args.format}")
    done = missing.implemented()
    issues = read_state(repo)
    by_name = {i["title"][len("Card: "):]: i for i in issues.values() if i["title"].startswith("Card: ")}

    wanted = [c for c in formats[args.format].values() if slug(c["name"]) not in done]
    played = len(formats[args.format])
    weight = sum(c["score"] for c in formats[args.format].values())
    covered = sum(c["score"] for c in formats[args.format].values() if slug(c["name"]) in done)
    print(f"{args.format}: {played - len(wanted)} of {played} played cards implemented "
          f"({100 * covered / weight:.1f}% by how much they are played); {len(wanted)} missing")

    create, relabel = [], []
    for card in wanted:
        labels, entries = labels_for(card["name"], formats)
        issue = by_name.get(card["name"])
        if issue is None:
            create.append((card["name"], labels, entries))
        elif issue["state"] == "OPEN" and not labels <= issue["labels"]:
            relabel.append((issue["number"], labels - issue["labels"]))
    stale = [i for n, i in by_name.items() if i["state"] == "OPEN" and slug(n) in done]
    print(f"{len(create)} issues to open, {len(relabel)} to label, {len(stale)} to close")
    if args.dry_run:
        for name, labels, _ in create[:25]:
            print(f"  would open {title(name)!r} {sorted(labels)}")
        return
    ensure_labels(repo, set().union(*(labels for _, labels, _ in create), *(l for _, l in relabel)))
    create = create[:args.limit] if args.limit else create
    cards = scryfall_cards([name for name, _, _ in create])
    # GitHub allows about 80 content-creating requests a minute and 500 an hour: a run of more than an hour's worth
    # keeps under the hourly limit instead of running into it.
    pace = 1.2 if len(create) <= 400 else 7.5
    for index, (name, labels, entries) in enumerate(create, 1):
        made = gh("api", "-X", "POST", f"repos/{repo}/issues", "--input", "-",
                  body={"title": title(name), "body": card_issue_body(name, cards.get(name), entries),
                        "labels": sorted(labels)})
        print(f"[{index}/{len(create)}] #{made['number']} {name}")
        time.sleep(pace)
    for number, labels in relabel:
        gh("api", "-X", "POST", f"repos/{repo}/issues/{number}/labels", "--input", "-", body={"labels": sorted(labels)})
    for issue in stale:
        gh("api", "-X", "POST", f"repos/{repo}/issues/{issue['number']}/comments", "--input", "-",
           body={"body": "The card is implemented."})
        gh("api", "-X", "PATCH", f"repos/{repo}/issues/{issue['number']}", "--input", "-",
           body={"state": "closed", "state_reason": "completed"})
        print(f"closed #{issue['number']}")


def available(args, repo):
    """The format's missing cards nobody holds live, in the format's order: (entry, issue or None, holder)."""
    formats = formats_of()
    if args.format not in formats:
        sys.exit(f"no docs/formats/{args.format}.json; run format-scrape.py {args.format}")
    done = done_anywhere()
    issues = read_state(repo)
    by_name = {i["title"][len("Card: "):]: i for i in issues.values() if i["title"].startswith("Card: ")}
    me = claimant(args)
    result = []
    for card in formats[args.format].values():
        if slug(card["name"]) in done:
            continue
        issue = by_name.get(card["name"])
        if issue and issue["state"] != "OPEN":
            continue
        held = holder(issue) if issue else None
        if held and held[0] != me:
            continue
        result.append((card, issue, held))
    return result, me


def next_cards(args, repo):
    cards, me = available(args, repo)
    cards = cards[:args.n]
    if args.json:
        print(json.dumps([{"name": c["name"], "score": c["score"], "issue": i and i["number"],
                           "yours": bool(h)} for c, i, h in cards], indent=1))
        return
    for card, issue, held in cards:
        print(f"{card['score']:6.2f}%  {card['name']}  " + (f"#{issue['number']}" if issue else "(no issue yet)")
              + ("  (yours)" if held else ""))
    if not cards:
        print(f"nothing left to take in {args.format}")


def find_or_open(repo, issues, name):
    """The open issue of a card, opening it first when there is none."""
    for issue in issues.values():
        if issue["title"] == title(name) and issue["state"] == "OPEN":
            return issue
    formats = formats_of()
    labels, entries = labels_for(name, formats)
    ensure_labels(repo, labels)
    card = scryfall_cards([name]).get(name)
    if card is None:
        sys.exit(f"Scryfall does not know {name!r}; spell it as the card is printed")
    made = gh("api", "-X", "POST", f"repos/{repo}/issues", "--input", "-",
              body={"title": title(name), "body": card_issue_body(name, card, entries), "labels": sorted(labels)})
    print(f"opened #{made['number']} for {name}")
    made.update(state="OPEN", labels=labels, events=[], from_comments=0)
    issues[made["number"]] = made
    return made


def take(repo, issues, me, name, branch_ok=False):
    """Claim one card: True when it is now ours, False when someone else holds it or has implemented it. `branch_ok`
    claims a card that is implemented only on a local branch - a pull request's under review, whose cards have no issue."""
    if slug(name) in (missing.implemented() if branch_ok else done_anywhere()):
        print(f"{name}: already implemented, here or on a local branch")
        return False
    issue = find_or_open(repo, issues, name)
    held = holder(issue)
    if held and held[0] != me:
        print(f"{name}: #{issue['number']} is held by {named(held[0])} until "
              f"{(held[2] + HOLD).strftime('%Y-%m-%d %H:%M')} UTC")
        return False
    taken_over = [e for e in issue["events"] if e[1] == "claim" and e[2] != me]
    note = f" (the earlier claim, by {named(taken_over[-1][2])}, ran out)" if taken_over and not held else ""
    login, _, tag = me.partition("/")
    gh("api", "-X", "POST", f"repos/{repo}/issues/{issue['number']}/comments", "--input", "-",
       body={"body": f"{CLAIM}\n{CLAIMANT.format(tag)}\nClaimed by {named(me)}{note}. Held 24 hours from the last "
                     f"push or commit on its pull request or `card/{slug(name)}` branch; claim again to renew."})
    fresh = read_issue(repo, issue)
    winner = holder(fresh)
    if winner and winner[0] != me:
        print(f"{name}: #{issue['number']} - {named(winner[0])} claimed it first")
        return False
    print(f"{name}: #{issue['number']} is yours (Closes #{issue['number']})")
    return True


def claimant(args):
    """This claimant: the signed-in account and this checkout's name."""
    tag = getattr(args, "claimant", None) or os.environ.get("CANTRIP_CLAIMANT") or os.path.basename(ROOT)
    return identity(gh("api", "user")["login"], re.sub(r"[^\w.-]", "-", tag))


def claim(args, repo):
    me = claimant(args)
    issues = read_state(repo)
    if args.names:
        sys.exit(0 if all([take(repo, issues, me, name, args.pr) for name in args.names]) else 3)
    if not args.format:
        sys.exit("name the cards to claim, or --format to claim the format's next free card")
    args.n = args.n or BATCH
    got = 0
    free = [card for card, _, mine in available(args, repo)[0] if not mine]  # one already ours: claim a new card
    for card in free:
        if got == args.n:
            break
        got += take(repo, issues, me, card["name"])
    if not free:
        print(f"nothing claimed: no card in {args.format} is free (every missing card is implemented on a branch or "
              f"held live by someone; `status` lists the holds)")
    else:
        print(f"{got} of {args.n} claimed; {'they are' if got != 1 else 'it is'} yours for 24 hours")
    sys.exit(0 if got == args.n else 3)


def release(args, repo):
    issues = read_state(repo)
    tag = claimant(args).partition("/")[2]
    for name in args.names:
        issue = next((i for i in issues.values() if i["title"] == title(name) and i["state"] == "OPEN"), None)
        if issue is None:
            print(f"{name}: no open issue")
            continue
        gh("api", "-X", "POST", f"repos/{repo}/issues/{issue['number']}/comments", "--input", "-",
           body={"body": f"{RELEASE}\n{CLAIMANT.format(tag)}\nReleased; the card is free."})
        print(f"{name}: #{issue['number']} released")


def log(args, repo):
    """Posts one progress event on a card's issue: its fixed text, filled with the one input it takes."""
    text, needs = EVENTS[args.event]
    value = getattr(args, needs) if needs else None
    if needs and value is None:
        sys.exit(f"{args.event} takes --{needs}")
    number = args.issue.lstrip("#")
    if not number.isdigit():  # a card's name: find its open issue (slower: reads every card issue)
        issue = next((i for i in read_state(repo).values() if i["title"] == title(args.issue) and i["state"] == "OPEN"), None)
        if issue is None:
            sys.exit(f"{args.issue}: no open issue")
        number = str(issue["number"])
    tag = claimant(args).partition("/")[2]
    body = f"{PROGRESS.format(args.event)}\n{CLAIMANT.format(tag)}\n" + text.format(**{needs: value} if needs else {})
    gh("api", "-X", "POST", f"repos/{repo}/issues/{number}/comments", "--input", "-", body={"body": body})
    print(f"#{number}: {args.event}")


TIMES = """query($owner:String!,$name:String!,$endCursor:String,$since:DateTime){repository(owner:$owner,name:$name){
issues(first:50,after:$endCursor,labels:["card"],filterBy:{since:$since}){pageInfo{hasNextPage endCursor}nodes{number
title state closedAt comments(last:60){nodes{body createdAt}}}}}}"""


def times(args, repo):
    """How long each card took, from the progress log on its issue: claim to commit (implementing), commit to review
    passed (waiting for and in review, fixes included), passed to landed, the whole, and the time it waited on the
    engine. Cards claimed since --days ago, newest first, then the medians."""
    since = (datetime.datetime.now(datetime.timezone.utc) - datetime.timedelta(days=args.days)).strftime("%Y-%m-%dT%H:%M:%SZ")
    rows = []
    for page in graphql(TIMES, repo, since=since):
        for issue in page["data"]["repository"]["issues"]["nodes"]:
            marks = {}
            blocked, since_blocked = 0.0, None
            for comment in issue["comments"]["nodes"]:
                at = when(comment["createdAt"])
                event = "claimed" if CLAIM in comment["body"] else (PROGRESS_RE.search(comment["body"]) or [None, None])[1]
                if event is None:
                    continue
                if event == "claimed" and "landed" in marks:
                    marks = {}  # claimed again after it landed: a new piece of work
                marks.setdefault(event, at)  # the first of each, but the last review verdict
                if event in ("review-passed", "landed"):
                    marks[event] = at
                if event == "blocked":
                    since_blocked = at
                elif event == "unblocked" and since_blocked:
                    blocked += (at - since_blocked).total_seconds()
                    since_blocked = None
            if "claimed" not in marks or "committed" not in marks:
                continue
            landed = marks.get("landed") or (when(issue["closedAt"]) if issue.get("closedAt") else None)
            minutes = lambda a, b: (b - a).total_seconds() / 60 if a and b else None
            rows.append((marks["claimed"], issue["number"], issue["title"][len("Card: "):],
                         minutes(marks["claimed"], marks["committed"]), minutes(marks["committed"], marks.get("review-passed")),
                         minutes(marks.get("review-passed"), landed), minutes(marks["claimed"], landed), blocked / 60,
                         "review-blocked" in marks))
    if not rows:
        print(f"no card with a progress log in the last {args.days} days")
        return
    fmt = lambda m: f"{m:6.1f}" if m is not None else "     -"
    print(f"{'card':34s} {'implement':>9s} {'review':>7s} {'land':>6s} {'total':>6s} {'engine':>7s}  (minutes)")
    for _, number, name, impl, review, land, total, engine, sent_back in sorted(rows, reverse=True):
        print(f"#{number:<4d} {name[:28]:28s} {fmt(impl):>9s} {fmt(review):>7s} {fmt(land):>6s} {fmt(total):>6s} "
              f"{fmt(engine) if engine else '      ':>7s}{'  sent back' if sent_back else ''}")
    for label, column in (("implement", 3), ("review", 4), ("land", 5), ("total", 6)):
        values = sorted(r[column] for r in rows if r[column] is not None)
        if values:
            print(f"median {label}: {values[len(values) // 2]:.1f} min over {len(values)} cards")


def status(args, repo):
    me = claimant(args)
    rows = []
    for issue in read_state(repo).values():
        held = holder(issue) if issue["state"] == "OPEN" else None
        if held:
            rows.append((held[0] != me, issue["number"], issue["title"], held))
    for mine, number, name, (user, since, last) in sorted(rows):
        print(f"#{number} {name}: {named(user)}{' (you)' if not mine else ''}, "
              f"free {(last + HOLD).strftime('%Y-%m-%d %H:%M')} UTC unless there is more work")
    if not rows:
        print("no live claims")


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--repo", help="owner/name; default is origin's")
    parser.add_argument("--as", dest="claimant",
                        help="the checkout claims are made as; default $CANTRIP_CLAIMANT, else the checkout's directory")
    commands = parser.add_subparsers(dest="command", required=True)
    one = commands.add_parser("sync")
    one.add_argument("format")
    one.add_argument("--dry-run", action="store_true")
    one.add_argument("--limit", type=int, default=0, help="open at most this many issues in this run")
    one = commands.add_parser("next")
    one.add_argument("format")
    one.add_argument("-n", type=int, default=BATCH)
    one.add_argument("--json", action="store_true")
    one = commands.add_parser("claim")
    one.add_argument("names", nargs="*")
    one.add_argument("--format", help="with no names, claim this format's next free cards")
    one.add_argument("-n", type=int, default=0, help=f"how many (default {BATCH})")
    one.add_argument("--pr", action="store_true",
                     help="the named cards are a pull request's under review: claim them though a local branch has them")
    one = commands.add_parser("release")
    one.add_argument("names", nargs="+")
    commands.add_parser("status")
    one = commands.add_parser("log", help="post a progress event on a card's issue (fixed text)")
    one.add_argument("issue", help="the issue number (#N from the claim), or the card's name (slower)")
    one.add_argument("event", choices=sorted(EVENTS))
    one.add_argument("--commit", help="for committed, fixes-committed, landed")
    one.add_argument("--findings", type=int, help="for review-blocked")
    one.add_argument("--rule", help="for blocked: the CR rule the engine lacks")
    one = commands.add_parser("times", help="how long each card took, from the progress logs")
    one.add_argument("--days", type=int, default=30)
    args = parser.parse_args()
    repo = repository(args.repo)
    {"sync": sync, "next": next_cards, "claim": claim, "release": release, "status": status, "log": log, "times": times}[args.command](args, repo)


if __name__ == "__main__":
    main()
