"""Where an ability's text is already implemented: every implemented card's Oracle text, split into abilities and their
parts, normalised into templates and matched, so a card agent reads the one card that already does a line instead of
searching for it.

    python tools/maintenance/ability-index.py "Card Name"        # each ability of a card in content/, and its prior art
    python tools/maintenance/ability-index.py --text "Scry 2."   # the same for text that has no card file yet
    python tools/maintenance/ability-index.py rank [--format F] [--top N]
                                                                 # the constructed pool against what is implemented

The index is built from content/*/cards and crates/cantrip-cards/src/cards on every run (under a second), so it is never
stale. An ability is a paragraph of Oracle text (a modal spell's bullets stay with it); its parts are its cost, its
trigger condition and each effect sentence; a keyword line is one ability per keyword. A template is the text with the
card's own name and "this creature" as ~ and reminder text and ability words dropped, then, level by level:

    exact     nothing more
    numbers   numbers and number words as N, every run of mana symbols as {M}
    types     also card types, colours and subtypes as TYPE, COL and SUB

An ability is matched at the most specific level any implemented card shares it at; failing that its parts are, and
failing that the nearest whole abilities by word overlap (TF-IDF cosine). Every match prints what differs between the
two texts, since a match is a model to change, never one to copy blind: "you" and "each opponent", "may", "another" and
"you control" are kept in every template because each one is different code.

`rank` reads Scryfall's Oracle Cards bulk file (scryfall.py: cached once a day in target/scryfall) and prints the cards legal
in the constructed formats that are not implemented, by how much of them is: every ability matched, every part matched,
or how many abilities are new; and the missing ability templates the most of them print.
"""

import collections
import difflib
import json
import math
import os
import re
import subprocess
import sys

import scryfall

sys.stdout.reconfigure(encoding="utf-8")
sys.stderr.reconfigure(encoding="utf-8")


def _root():
    try:
        out = subprocess.run(["git", "rev-parse", "--show-toplevel"], capture_output=True, text=True, check=True).stdout.strip()
        if out:
            return os.path.normpath(out)
    except (OSError, subprocess.CalledProcessError):
        pass
    return os.path.normpath(os.path.join(os.path.dirname(__file__), "..", ".."))


ROOT = _root()
CONTENT = os.path.join(ROOT, "content")
CARDS = os.path.join(ROOT, "crates", "cantrip-cards", "src", "cards")
MECHANICS = os.path.join(ROOT, "crates", "cantrip-mechanics", "src")
API = os.path.join(ROOT, "crates", "cantrip-api", "src")
SUBTYPES = os.path.join(ROOT, "crates", "cantrip-state", "src", "subtypes.rs")
ORACLE_BUILD = os.path.join(ROOT, "crates", "cantrip-oracle", "build.rs")
CONSTRUCTED = ["standard", "pioneer", "modern", "legacy", "vintage", "pauper", "commander"]
LEVELS = ["exact", "numbers", "types"]


def file_stem(name):
    return re.sub(r"[^a-z0-9]+", "_", name.replace("'", "").lower()).strip("_")


# ---------------------------------------------------------------------------------------------------------- templates

NUMBER_WORDS = r"\b(?:a|an|one|two|three|four|five|six|seven|eight|nine|ten|eleven|twelve|thirteen|fifteen|twenty|x|once|twice)\b"
CARD_TYPES = (r"\b(?:non)?(?:artifact|creature|enchantment|land|planeswalker|instant|sorcery|battle|permanent|spell|basic|"
              r"legendary|historic|kindred)s?\b")
COLORS = r"\b(?:non)?(?:white|blue|black|red|green|colorless|multicolored|monocolored)\b"
SELF = (r"\bthis (?:creature|artifact|enchantment|land|planeswalker|spell|card|permanent|equipment|aura|vehicle|battle|"
        r"saga|class|siege|token|room|case|spacecraft|planet)\b")


def _subtypes():
    """Every subtype the engine knows, from cantrip-state's subtypes.rs (its string literals), longest first."""
    words = set()
    try:
        for literal in re.findall(r'"([A-Z][A-Za-z\'\- ]+)"', open(SUBTYPES, encoding="utf-8").read()):
            if len(literal) > 2:
                words.add(literal.lower())
    except OSError:
        pass
    words -= {"and", "the", "you", "card", "token"}
    return re.compile(r"\b(?:" + "|".join(re.escape(w) for w in sorted(words, key=len, reverse=True)) + r")s?\b") if words else None


SUBTYPE_RE = _subtypes()


def self_names(names):
    """A card's own names as Oracle text says them: each face's full name, and a legendary's short name before the comma."""
    out = set()
    for n in names:
        if not n:
            continue
        out.add(n)
        if "," in n and len(n.split(",")[0]) > 2:
            out.add(n.split(",")[0])
    return sorted(out, key=len, reverse=True)


def clean(text, names):
    """Oracle text with the card's own name as ~ and reminder text gone."""
    for n in self_names(names):
        text = re.sub(r"(?<![\w'])" + re.escape(n) + r"(?![\w'])", "~", text)
    text = re.sub(SELF, "~", text, flags=re.I)
    return re.sub(r" ?\([^)]*\)", "", text)


MANA_RUN = r"(?:\{[^}]+\})+"
NUMBER = r"[+\-−]?\d+"
# What the numbers level replaces, as one scan in the order the replacements below make it: a mana run is taken whole
# before the digits in it, and a number word never overlaps either.
VALUE = re.compile(f"{MANA_RUN}|{NUMBER}|{NUMBER_WORDS}")


def _exact(text):
    s = re.sub(r"\s+", " ", text.lower()).strip(" .")
    return re.sub(r"^then,? ", "", s)  # sequencing within the ability, not what the sentence does


def template(text, level):
    s = _exact(text)
    if level == "exact":
        return s
    s = re.sub(MANA_RUN, "{M}", s)
    s = re.sub(NUMBER, "N", s)
    s = re.sub(NUMBER_WORDS, "N", s)
    s = re.sub(r"\b(\w\w+?)(?:e?s)\b", r"\1", s)  # plurals: "a card" and "two cards" are one template
    if level == "numbers":
        return s
    if SUBTYPE_RE:
        s = SUBTYPE_RE.sub("SUB", s)
    s = re.sub(COLORS, "COL", s)
    s = re.sub(CARD_TYPES, "TYPE", s)
    s = re.sub(r"\bTYPE(?: TYPE)+\b", "TYPE", s)
    return s


def values(text):
    """What the numbers level of `template` replaces, in order: each mana run ("{2}{r}"), signed number ("+1", "-2")
    and number word ("two", "x", "a"), lowercase, so a template's {M} and N can be given back their values."""
    return VALUE.findall(_exact(text))


# ---------------------------------------------------------------------------------------------------------- abilities

TEXT_KEYWORDS = [  # whole lines (name as ~, no reminder text) the core plays as a keyword of its own
    (r"(?:this spell|~) can't be countered\.", "CantBeCountered"),
    (r"~ can't be blocked\.", "CantBeBlocked"),
    (r"you can't spend mana to cast (?:this spell|~)\.", "CantSpendManaToCast"),
    (r"you may cast ~ from your graveyard\.", "CastFromYourGraveyard"),
]


ABILITY_WORD = re.compile(r"^[A-Z][\w' ,\-]*? — (?=[A-Z{])")
TRIGGER = re.compile(r"^(when|whenever|at) ", re.I)


class Ability:
    """One ability of a card: its text (name as ~, no reminder text) and its parts, each a (kind, text)."""

    def __init__(self, text, parts, keyword=None):
        self.text, self.parts, self.keyword = text, parts, keyword

    def templates(self, level):
        return template(self.text, level)


def is_keyword_line(line):
    if line.endswith(":") or ":" in line or len(line) > 70:
        return False
    if line.rstrip().endswith(".") and "—" not in line:
        return False
    if line.startswith(("Choose", "When", "Whenever", "At ", "As ", "If ", "You ", "Each ", "Target ", "Draw", "Create")):
        return False
    return True


def keyword_parts(line):
    """'Flying, vigilance' -> ['Flying', 'vigilance']; 'Ward—Pay 3 life.' and 'Protection from red' stay whole."""
    if "—" in line:
        return [line]
    return [p.strip() for p in re.split(r",(?![^{]*\})", line) if p.strip()]


def keyword_name(part):
    """The keyword's words without its cost or number: 'Kicker {2}{R}' -> 'kicker', 'Toxic 2' -> 'toxic'."""
    head = re.split(r"[—{]|\s\d|\sX\b|\sfrom\b|—", part)[0]
    return head.strip(" .").lower()


def split_abilities(oracle, names):
    out = []
    text = clean(oracle, names)
    blocks = []
    for line in (l.strip() for l in text.split("\n")):
        if not line or line == "//":
            continue
        if line.startswith("•") and blocks:
            blocks[-1].append(line)
        else:
            blocks.append([line])
    for block in blocks:
        head = ABILITY_WORD.sub("", block[0])
        if len(block) == 1 and is_keyword_line(head):
            for part in keyword_parts(head):
                out.append(Ability(part, [("keyword", part)], keyword=keyword_name(part)))
            continue
        parts = []
        rest = head
        cost = re.match(r'^([^:"]+):\s*(.+)$', head)
        if cost and not TRIGGER.match(head) and not head.lower().startswith("choose"):
            parts.append(("cost", cost.group(1)))
            rest = cost.group(2)
        elif TRIGGER.match(head) and "," in head:
            condition, rest = head.split(",", 1)
            intervening = re.match(r"\s*(if [^,]+),\s*(.+)$", rest, re.S)  # CR 603.4: part of the condition
            if intervening:
                condition, rest = f"{condition}, {intervening.group(1)}", intervening.group(2)
            parts.append(("trigger", condition))
        for sentence in re.split(r"(?<=\.)\s+(?=[A-Z~])", rest):
            if sentence.strip():
                # "Choose one -" over bullets is the modal frame (CR 700.2), matched against other modal abilities.
                modal = len(block) > 1 and re.match(r"choose\b", sentence.strip(), re.I)
                parts.append(("modal" if modal else "effect", sentence.strip()))
        for bullet in block[1:]:  # each mode's sentences, without its name ("Date Night - ", CR 700.2h)
            mode = ABILITY_WORD.sub("", bullet.lstrip("• ").strip())
            parts += [("effect", s.strip()) for s in re.split(r"(?<=\.)\s+(?=[A-Z~])", mode) if s.strip()]
        # Each "target ..." phrase is a part of its own: a target filter is the code most often copied between cards
        # whose effects differ ("target green or white creature" is Deathmark's and Rending Volley's at once).
        for kind, text in list(parts):
            if kind in ("effect", "cost"):
                parts += [("target", phrase) for phrase in target_phrases(text)]
        out.append(Ability(" ".join([head] + block[1:]), parts))
    return out


TARGET = re.compile(r"\b((?:(?:up to |one or )?(?:one|two|three|four|five|x|\d+) |one or more |any number of )?"
                    r"(?:other |another )?targets? )([^.,;:]+)", re.I)
# Where a target's noun phrase ends: the verb or preposition that follows it.
TARGET_END = re.compile(
    r" (?:to|from|into|onto|on|until|this|for|as|is|are|and|unless|that|each|can|can't|doesn't|its owner's|their owners'|"
    r"(?:get|gain|lose|deal|discard|draw|sacrifice|mill|put|reveal|shuffle|search|exile|return|create|attack|block|"
    r"fight|become|transform|untap|tap|phase|explore|connive|scrie|surveil)s?|has|have)\b.*$", re.I)


def target_phrases(text):
    """The target phrases of a sentence, each cut where its noun phrase ends: "target creature an opponent controls",
    "up to two target cards from graveyards" -> "up to two target cards", "Target player discards" -> "Target player"."""
    return [(m.group(1) + TARGET_END.sub("", m.group(2))).strip() for m in TARGET.finditer(text)]


def card_texts(card):
    """Every (name, Oracle text) of a content card: its faces, halves, adventure and omen."""
    out = []

    def walk(node):
        if isinstance(node, dict):
            if isinstance(node.get("oracleText"), str):
                out.append((node.get("name", ""), node["oracleText"]))
            for key, value in node.items():
                if key != "oracleText":
                    walk(value)
        elif isinstance(node, list):
            for value in node:
                walk(value)

    walk(card)
    return out


def card_abilities(card):
    texts = card_texts(card)
    names = [n for n, _ in texts] + [card.get("name", "")]
    return [a for _, t in texts for a in split_abilities(t, names)]


# ---------------------------------------------------------------------------------------------------------- the index

def load_content():
    """{name: card json} for every card in content/."""
    cards = {}
    for dataset in sorted(os.listdir(CONTENT)):
        folder = os.path.join(CONTENT, dataset, "cards")
        if not os.path.isdir(folder):
            continue
        for file in os.listdir(folder):
            if not file.endswith(".json"):
                continue
            text = open(os.path.join(folder, file), encoding="utf-8-sig").read()
            body = "\n".join(l for l in text.split("\n") if not l.strip().startswith("//"))
            try:
                card = json.loads(body)
            except ValueError:
                continue
            if "name" in card:
                cards[card["name"]] = card
    return cards


def card_file(name):
    path = os.path.join(CARDS, file_stem(name) + ".rs")
    return path if os.path.exists(path) else None


_uses_cache = {}


def card_uses(name):
    """What a card's file builds with: its cantrip_mechanics imports and the Game verbs it calls."""
    if name in _uses_cache:
        return _uses_cache[name]
    path = card_file(name)
    uses = []
    if path:
        src = open(path, encoding="utf-8").read()
        for tree in re.findall(r"use cantrip_mechanics::([^;]+);", src):
            uses += [u for u in expand_use(re.sub(r"\s+", "", tree)) if not u.endswith("::self")]
        verbs = []
        for verb in re.findall(r"\bg\.(\w+)\(", src):
            if verb not in verbs and verb not in ("state", "query", "ask"):
                verbs.append(verb)
        uses += ["g." + v for v in verbs[:8]]
    _uses_cache[name] = uses
    return uses


def expand_use(tree, prefix=""):
    """'triggers::{LANDFALL,THIS_ENTERS}' -> ['triggers::LANDFALL', 'triggers::THIS_ENTERS']."""
    items, depth, start = [], 0, 0
    for i, c in enumerate(tree):
        depth += (c == "{") - (c == "}")
        if c == "," and depth == 0:
            items.append(tree[start:i])
            start = i + 1
    items.append(tree[start:])
    out = []
    for item in filter(None, items):
        brace = item.find("{")
        if brace >= 0 and item.endswith("}"):
            out += expand_use(item[brace + 1:-1], prefix + item[:brace])
        else:
            out.append(prefix + item)
    return out


def core_keywords():
    try:
        src = open(ORACLE_BUILD, encoding="utf-8").read()
        block = re.search(r"KEYWORD_WORDS[^=]*=\s*\[(.*?)\];", src, re.S).group(1)
        return set(re.findall(r'"([^"]+)"', block))
    except (OSError, AttributeError):
        return set()


_sources = {}


def _crate_sources():
    """(crate, module path, source) of every file of cantrip-mechanics, then cantrip-api."""
    if not _sources:
        for crate, folder in (("cantrip_mechanics", MECHANICS), ("cantrip_api", API)):
            for file in sorted(os.listdir(folder)):
                if file.endswith(".rs"):
                    module = "" if file == "lib.rs" else file[:-3] + "::"
                    _sources[(crate, module)] = open(os.path.join(folder, file), encoding="utf-8").read()
    return _sources


def mechanics_for(keyword):
    """Where a keyword is defined: a cantrip-mechanics module named for it, else a public function, constant,
    re-export or CardDef field of that name in cantrip-mechanics or cantrip-api ("ward", "equip", "convoke")."""
    stem = file_stem(keyword)
    if not stem:
        return None
    if os.path.exists(os.path.join(MECHANICS, stem + ".rs")):
        return f"cantrip_mechanics::{stem}"
    name = r"(" + stem.upper() + r"|" + stem + r")\b"
    item = re.compile(r"pub (?:(?:const )?fn|static|const|use [\w:]+::) ?" + name + r"(?!::)")
    field = re.compile(r"^\s*pub " + stem + r": ", re.M)
    for (crate, module), src in _crate_sources().items():
        m = item.search(src)
        if m:
            return f"{crate}::{module}{m.group(1)}"
    for (crate, module), src in _crate_sources().items():
        if field.search(src):
            return (f"the `{stem}` field ({crate}::{module.rstrip(':')}), set in the card's Rust file (`{stem}: Some(..)`, as an"
                    f" implemented card with it does): the content JSON and the oracle build don't carry it")
    if "_" in stem:  # "Empower Jace" is empower's
        return mechanics_for(stem.split("_")[0])
    return None


class Index:
    def __init__(self, exclude=()):
        self.content = load_content()
        exclude = {e.lower() for e in exclude}
        self.whole = {level: collections.defaultdict(list) for level in LEVELS}
        self.part = {level: collections.defaultdict(list) for level in LEVELS}
        self.texts = {}  # numbers-level whole template -> (card, ability text) for similarity
        self.modal = set()  # the templates of self.texts that are modal abilities
        self.cards = {}
        for name, card in self.content.items():
            if name.lower() in exclude:
                continue
            abilities = card_abilities(card)
            self.cards[name] = abilities
            for ab in abilities:
                for level in LEVELS:
                    self.whole[level][template(ab.text, level)].append((name, ab.text))
                    for kind, text in ab.parts:
                        self.part[level][(kind, template(text, level))].append((name, text))
                self.texts.setdefault(template(ab.text, "numbers"), (name, ab.text))
                if any(kind == "modal" for kind, _ in ab.parts):
                    self.modal.add(template(ab.text, "numbers"))
        self.core = core_keywords()
        self._idf = None

    # Matches are listed simplest card first: fewest abilities, then a card with a file of its own.
    def _rank(self, hits):
        seen, out = set(), []
        for name, text in sorted(hits, key=lambda h: (len(self.cards.get(h[0], ())), card_file(h[0]) is None, h[0])):
            if name not in seen:
                seen.add(name)
                out.append((name, text))
        return out

    def match_whole(self, ab):
        for level in LEVELS:
            hits = self.whole[level].get(template(ab.text, level))
            if hits:
                return level, self._rank(hits)
        return None, []

    def match_part(self, kind, text):
        for level in LEVELS:
            hits = self.part[level].get((kind, template(text, level)))
            if hits:
                return level, self._rank(hits)
        return None, []

    def keyword_home(self, ab):
        for pattern, keyword in TEXT_KEYWORDS:
            if re.fullmatch(pattern, ab.text.strip(), re.I):
                return f"core keyword {keyword} (in the content JSON's keywords; no code)"
        if ab.keyword is None:
            return None
        if ab.keyword == "toxic":
            return "core keyword toxic N (\"Toxic N\" in the content JSON's keywords; no code)"
        if ab.keyword == "riot":
            return "core keyword riot (\"Riot\" in the content JSON's keywords; no code)"
        if ab.keyword in self.core:
            return "core keyword (Keywords::of in the content JSON; no code)"
        return mechanics_for(ab.keyword)

    def nearest(self, text, count=3, modal=False):
        """The implemented abilities nearest a text by TF-IDF cosine over words and word pairs; only the modal ones
        with `modal`."""
        if self._idf is None:
            self._docs = {t: self._grams(t) for t in self.texts}
            df = collections.Counter(g for grams in self._docs.values() for g in set(grams))
            n = len(self._docs)
            self._idf = {g: math.log((1 + n) / (1 + c)) + 1 for g, c in df.items()}
            self._vecs = {t: self._vec(grams) for t, grams in self._docs.items()}
        q = self._vec(self._grams(template(text, "numbers")))
        scored = []
        for t, v in self._vecs.items():
            if modal and t not in self.modal:
                continue
            dot = sum(w * v.get(g, 0.0) for g, w in q.items())
            if dot > 0:
                scored.append((dot, t))
        scored.sort(reverse=True)
        return [(score, *self.texts[t]) for score, t in scored[:count]]

    def nearest_modal(self, text, count=2):
        """The implemented modal abilities most like a modal text: most modes that open with the same two words
        ("counter target", "destroy target"), what a mode does, then the nearest by TF-IDF."""
        heads = self._mode_heads(template(text, "numbers"))
        tfidf = {t: s for s, *_, t in ((s, n, x, template(x, "numbers")) for s, n, x in self.nearest(text, 10 ** 6, modal=True))}
        scored = sorted(((len(heads & self._mode_heads(t)), tfidf.get(t, 0.0), t) for t in self.modal), reverse=True)
        return [(shared, *self.texts[t]) for shared, _, t in scored[:count] if shared or tfidf.get(t, 0.0) > 0]

    @staticmethod
    def _mode_heads(t):
        modes = (re.sub(r"^[^—]*— ", "", m.strip()) for m in t.split("•")[1:])  # without a mode's name
        return {" ".join(m.split()[:2]) for m in modes}

    @staticmethod
    def _grams(t):
        words = re.findall(r"[~\w{}/+]+", t)
        return words + [a + " " + b for a, b in zip(words, words[1:])]

    def _vec(self, grams):
        counts = collections.Counter(grams)
        v = {g: c * self._idf.get(g, 1.0) for g, c in counts.items()}
        norm = math.sqrt(sum(x * x for x in v.values())) or 1.0
        return {g: x / norm for g, x in v.items()}


def diff(mine, theirs):
    """What differs between two texts, word by word: 'two -> three; + until end of turn'."""
    a, b = mine.split(), theirs.split()
    out = []
    matcher = difflib.SequenceMatcher(None, [w.lower() for w in b], [w.lower() for w in a], autojunk=False)
    for op, i1, i2, j1, j2 in matcher.get_opcodes():
        if op == "replace":
            out.append(f"{' '.join(b[i1:i2])} -> {' '.join(a[j1:j2])}")
        elif op == "delete":
            out.append(f"- {' '.join(b[i1:i2])}")
        elif op == "insert":
            out.append(f"+ {' '.join(a[j1:j2])}")
    return "; ".join(out)


# ---------------------------------------------------------------------------------------------------------- reports

def describe(name, mine, theirs, indent, label="", only=None):
    """A match: the card, its file, what it builds with (only the items naming `only`, when given) and how it differs."""
    path = card_file(name)
    where = os.path.relpath(path, ROOT).replace("\\", "/") if path else "no file (characteristics only)"
    line = f"{indent}{label}{name}  {where}"
    uses = [u for u in card_uses(name) if only is None or only in u]
    if uses:
        line += "\n" + indent + "    uses " + ", ".join(uses)
    d = diff(mine, theirs)
    if d:
        line += "\n" + indent + f"    differs: {d}  (theirs: {theirs!r})"
        # A model that acts on one object, for a text that acts on several: its loop must not be copied.
        if re.search(r"\b(?:one|a|an|target)\b[^;]*-> [^;]*\b(?:two|three|four|five|each|all|any number|cards|creatures|permanents)\b", d):
            line += "\n" + indent + "    yours acts on more than theirs: act on them all at once (exile_all, destroy_all ... - CR 101.4, 608.2), never one by one"
    return line


def matched(level, hits):
    return f"= {level} match, {len(hits)} card{'s' if len(hits) > 1 else ''}:"


def report(index, abilities):
    """Each ability and the implemented cards that already do it, simplest first, as new-card.py prints it."""
    lines, seen = [], {}
    for i, ab in enumerate(abilities, 1):
        if ab.text in seen:  # the same ability on another face
            lines.append(f"[{i}] {ab.text}  - as [{seen[ab.text]}]")
            continue
        seen[ab.text] = i
        lines.append(f"[{i}] {ab.text}")
        home = index.keyword_home(ab)
        if home:
            lines.append(f"    keyword: {home}")
            if home.startswith("core"):
                continue
        level, hits = index.match_whole(ab)
        if hits:
            lines.append("    " + matched(level, hits))
            for name, text in hits[:2]:
                lines.append(describe(name, ab.text, text, "      "))
            continue
        if home:
            continue
        lines.append("    no implemented card prints this ability; by part:")
        new = False
        for kind, text in ab.parts:
            lines.append(f"    {kind}: {text}")
            if kind == "modal":  # every modal card shares the frame; the ones like this one are the model
                level, hits = index.match_part(kind, text)
                near = index.nearest_modal(ab.text)
                lines.append(f"      {len(hits)} modal abilities say {text!r}" + ("; the most like this one:" if near else ""))
                for shared, name, theirs in near:
                    lines.append(describe(name, ab.text, theirs, "        ", f"{shared} mode{'' if shared == 1 else 's'} alike: "))
                continue
            level, hits = index.match_part(kind, text)
            if hits:
                lines.append("      " + matched(level, hits))
                # A part many cards print needs one model, and of a common trigger only the matcher it uses.
                common = len(hits) > 5
                if common and kind == "trigger":  # a model that names its matcher (`triggers::THIS_ENTERS`)
                    hits = sorted(hits, key=lambda h: not any(u.startswith("triggers::") for u in card_uses(h[0])))
                for name, theirs in hits[:1 if common else 2]:
                    lines.append(describe(name, text, theirs, "        ", only="triggers::" if common and kind == "trigger" else None))
            else:
                lines.append("      new")
                new = True
        near = [n for n in index.nearest(ab.text) if n[0] >= 0.5] if new else []
        if near:
            lines.append("    nearest abilities:")
            for score, name, text in near:
                lines.append(describe(name, ab.text, text, "      ", f"{score:.2f} "))
    return "\n".join(lines)


def rank(args):
    formats = CONSTRUCTED
    if "--format" in args:
        formats = [args[args.index("--format") + 1]]
    top = int(args[args.index("--top") + 1]) if "--top" in args else 40
    index = Index()
    done = {n.lower() for n in index.content}
    tiers = collections.Counter()
    missing = collections.defaultdict(set)
    total = 0
    for d in scryfall.oracle_cards():
        if d.get("set_type") in ("funny", "memorabilia", "alchemy"):
            continue
        if not any(d.get("legalities", {}).get(f) in ("legal", "restricted") for f in formats):
            continue
        faces = d.get("card_faces") or [d]
        if faces[0]["name"].lower() in done or d["name"].lower() in done:
            continue
        total += 1
        names = [f["name"] for f in faces] + [d["name"]]
        abilities = [a for f in faces for a in split_abilities(f.get("oracle_text") or "", names)]
        new = []
        for ab in abilities:
            if ab.keyword is not None and index.keyword_home(ab):
                continue
            if not index.match_whole(ab)[1]:
                new.append(ab)
        if not abilities:
            tiers["vanilla"] += 1
        elif not new:
            tiers["every ability matched"] += 1
        elif all(index.match_part(k, t)[1] for ab in new for k, t in ab.parts):
            tiers["every part matched"] += 1
        else:
            tiers[f"{min(len(new), 3)}{'+' if len(new) >= 3 else ''} new"] += 1
        for ab in new:
            missing[template(ab.text, "types")].add(d["name"])
    print(f"{total} cards legal in {'/'.join(formats)} are not implemented ({len(done)} cards are):")
    for tier in ["vanilla", "every ability matched", "every part matched", "1 new", "2 new", "3+ new"]:
        print(f"  {tier:24s} {tiers[tier]:6d}  {tiers[tier] / max(total, 1):6.1%}")
    print(f"\nthe {top} missing ability templates the most of them print (cards, template):")
    for t, names in sorted(missing.items(), key=lambda kv: -len(kv[1]))[:top]:
        print(f"  {len(names):5d}  {t[:110]}   e.g. {sorted(names)[0]}")


def main():
    args = sys.argv[1:]
    if not args:
        sys.exit(__doc__)
    if args[0] == "rank":
        return rank(args[1:])
    if args[0] == "--text":
        text = " ".join(args[1:]).replace("\\n", "\n")
        print(report(Index(), split_abilities(text, [])))
        return
    index = Index(exclude=[args[0]])
    card = next((c for n, c in index.content.items() if n.lower() == args[0].lower()), None)
    if card is None:
        sys.exit(f"no card named {args[0]!r} in content/; use --text for one that has none yet")
    print(report(index, card_abilities(card)))


if __name__ == "__main__":
    main()
