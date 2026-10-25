"""The card-text vectors (docs/ai-player-plan.md, "Network" > "Inputs"): one row of DIM numbers per card name, which
the network's card embedding starts from, so that a card it never trained on still means something.

Nothing here is checked in. The names are the card index a shard was recorded with (the engine's own cards,
`cantrip_ai::CardIndex`); a name's text is read from content/*/cards/*.json (and the dungeons): each face's type line,
mana cost and Oracle text, with the card's own names replaced by a placeholder, so that "When Aang enters" and "When
Sokka enters" read alike. A name with no text (a token, an emblem) has a row of zeros.

The vectors are a TF-IDF over words and word pairs, reduced by a truncated SVD, each row L2-normalised: no download,
the same bytes every run. The reduction is a `Basis` (the vocabulary, its IDF weights and the SVD's directions), fitted
once, when a network is first trained, and kept in its checkpoint. A later generation's new cards are projected into
the same basis (`project`), so the rows a network trained on never move.

    python -m cantrip_ai.embed [--names cards.json]   # how many names have text, and the basis fitted to them
"""

from __future__ import annotations

import argparse
import glob
import json
import math
import re
from collections import Counter
from dataclasses import dataclass
from pathlib import Path

import numpy as np

DIM = 64
VERSION = 1
MODEL = f"tfidf-svd-{VERSION}"
REPO = Path(__file__).resolve().parents[2]
PLACEHOLDER = "cardname"
FACES = ("backFace", "adventure", "prepare", "omen")


def face_text(face: dict, names: list) -> str:
    text = "\n".join(str(face.get(k, "")) for k in ("type", "manaCost", "oracleText"))
    if face.get("loyalty") is not None:
        text += f"\nloyalty {face['loyalty']}"
    for name in sorted(names, key=len, reverse=True):
        if name:
            text = text.replace(name, PLACEHOLDER)
    return text


def texts(root: Path = REPO) -> dict[str, str]:
    """The text of every name the content gives one: each face of a card (front, back, adventure, prepare, omen, each
    half), a split card's whole name, and each dungeon."""
    out = {}
    for path in sorted(glob.glob(str(root / "content" / "*" / "cards" / "*.json"))):
        with open(path, encoding="utf-8") as f:
            card = json.load(f)
        faces = [card] + [card[k] for k in FACES if isinstance(card.get(k), dict)] + list(card.get("halves", []))
        # A card's own names, and the short name before a comma ("Aang" for "Aang, at the Crossroads").
        names = set()
        for face in faces:
            names.add(face["name"])
            names.add(face["name"].split(",")[0])
        if card.get("halves"):
            out[card["name"]] = "\n".join(face_text(h, list(names)) for h in card["halves"])
            faces = faces[1:]
        for face in faces:
            out.setdefault(face["name"], face_text(face, list(names)))
    for path in sorted(glob.glob(str(root / "content" / "*" / "dungeons" / "*.json"))):
        with open(path, encoding="utf-8") as f:
            dungeon = json.load(f)
        text = "dungeon\n" + "\n".join(f"{room['name']}: {room.get('text', '')}" for room in dungeon.get("rooms", []))
        out.setdefault(dungeon["name"], text)
    if not out:
        raise SystemExit(f"no card content under {root / 'content'}: run from a checkout of the repository "
                         "(in a container, mount it and put its ai/ on PYTHONPATH)")
    return out


WORD = re.compile(r"\{[^}]*\}|[+-]?\d+/[+-]?\d+|[a-z0-9']+|[^\sa-z0-9]")


def tokens(text: str) -> list:
    words = WORD.findall(text.lower())
    return words + [f"{a} {b}" for a, b in zip(words, words[1:])]


@dataclass
class Basis:
    """A fitted TF-IDF and SVD: `vocab[i]`'s IDF weight is `idf[i]`, and `components` [len(vocab), dim] takes a
    normalised TF-IDF row to the SVD's directions."""

    vocab: list
    idf: np.ndarray
    components: np.ndarray
    model: str = MODEL

    @property
    def dim(self) -> int:
        return self.components.shape[1]

    def state(self) -> dict:
        """As a checkpoint keeps it."""
        return dict(model=self.model, vocab=list(self.vocab), idf=self.idf.astype(np.float32),
                    components=self.components.astype(np.float32))

    @staticmethod
    def from_state(state: dict) -> Basis:
        return Basis(list(state["vocab"]), np.asarray(state["idf"], np.float64), np.asarray(state["components"], np.float64),
                     state.get("model", MODEL))


def _tfidf(docs: list, index: dict, idf: np.ndarray) -> np.ndarray:
    x = np.zeros((len(docs), len(index)))
    for r, d in enumerate(docs):
        for w, c in d.items():
            if w in index:
                x[r, index[w]] = 1.0 + math.log(c)
    x *= idf
    x /= np.maximum(np.linalg.norm(x, axis=1, keepdims=True), 1e-12)
    return x


def _normalised(u: np.ndarray) -> np.ndarray:
    norms = np.linalg.norm(u, axis=1, keepdims=True)
    return u / np.where(norms == 0, 1.0, norms)


def fit(corpus: list, dim: int = DIM) -> Basis:
    """The basis of `corpus`: its words and pairs in two texts or more, and the top `dim` singular directions of their
    TF-IDF, found through the eigendecomposition of X X^T (the texts are few), each direction's sign fixed so that its
    largest entry over the corpus is positive."""
    docs = [Counter(tokens(t)) for t in corpus]
    df = Counter(w for d in docs for w in d)
    vocab = sorted(w for w, c in df.items() if c >= 2)
    n = len(corpus)
    idf = np.array([math.log((1 + n) / (1 + df[w])) + 1.0 for w in vocab])
    x = _tfidf(docs, {w: i for i, w in enumerate(vocab)}, idf)
    values, vectors = np.linalg.eigh(x @ x.T)
    order = np.argsort(-values, kind="stable")[:dim]
    sigma = np.sqrt(np.maximum(values[order], 0.0))
    u = vectors[:, order] * sigma
    signs = np.sign(u[np.argmax(np.abs(u), axis=0), np.arange(u.shape[1])])
    signs[signs == 0] = 1.0
    # X = U S V^T, so a row's coordinates U S are X V, and V = X^T U / S: what `project` multiplies a new row by.
    components = (x.T @ (vectors[:, order] * signs)) / np.where(sigma == 0, 1.0, sigma)
    components[:, sigma == 0] = 0.0
    if components.shape[1] < dim:
        components = np.pad(components, ((0, 0), (0, dim - components.shape[1])))
    return Basis(vocab, idf, components)


def project(basis: Basis, corpus: list) -> np.ndarray:
    """`corpus` in `basis`: [len(corpus), dim], each row L2-normalised (zeros for a text with none of its words)."""
    docs = [Counter(tokens(t)) for t in corpus]
    x = _tfidf(docs, {w: i for i, w in enumerate(basis.vocab)}, basis.idf)
    return _normalised(x @ basis.components)


def rows(names: list, basis: Basis | None = None, known: dict | None = None) -> tuple[np.ndarray, Basis]:
    """The table for the index `names` (`names[i]` is row `i + 1`; row 0, the unknown card, is zeros): each name with
    text projected into `basis`, fitted here to those texts when there is none; a name with no text is zeros."""
    known = texts() if known is None else known
    have = [i for i, n in enumerate(names) if n in known]
    corpus = [known[names[i]] for i in have]
    if basis is None:
        basis = fit(corpus)
    table = np.zeros((len(names) + 1, basis.dim), np.float32)
    if have:
        table[np.array(have) + 1] = project(basis, corpus)
    return table, basis


def extend(table: np.ndarray, names: list, basis: Basis | None, known: dict | None = None) -> np.ndarray:
    """`table`, the rows of `names[:len(table) - 1]`, with a row for each name after them: projected into `basis`, or
    zeros where there is no basis (or no text). The rows already there are kept as they are."""
    old = table.shape[0] - 1
    if old >= len(names):
        return table[: len(names) + 1]
    out = np.zeros((len(names) + 1, table.shape[1]), np.float32)
    out[: old + 1] = table
    if basis is not None:
        known = texts() if known is None else known
        new = [i for i in range(old, len(names)) if names[i] in known]
        if new:
            out[np.array(new) + 1] = project(basis, [known[names[i]] for i in new])
    return out


LEGACY_COSINE = 0.99


def legacy_basis(names: list, table: np.ndarray, log=print, known: dict | None = None) -> Basis | None:
    """The basis an older checkpoint's table was made with, which it did not keep: fitted again to its names' texts
    as the content has them now, and used only if it remakes the checkpoint's rows in the same directions (every row
    with text at a cosine of `LEGACY_COSINE` or more: an Oracle text corrected since moves a row a little, not the
    basis). The checkpoint's own rows are kept either way; the basis only places the new cards. `None` otherwise: the
    new cards then start at the unknown card's zeros."""
    known = texts() if known is None else known
    remade, basis = rows(names, None, known)
    if remade.shape != table.shape:
        log(f"embed: the checkpoint's table is {table.shape}, its names make {remade.shape}; new cards start at zeros")
        return None
    norms = np.linalg.norm(remade, axis=1) * np.linalg.norm(table, axis=1)
    both = norms > 0
    worst = float(((remade * table).sum(1)[both] / norms[both]).min()) if both.any() else 1.0
    if worst < LEGACY_COSINE:
        log(f"embed: today's card texts do not remake the checkpoint's table (a row at cosine {worst:.3f}); "
            "new cards start at zeros")
        return None
    log(f"embed: the checkpoint's basis fitted again from its {len(names)} names (every row at cosine {worst:.4f} or more)")
    return basis


def main():
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("--names", help="a cards.json ({'names': [...]}); default: every name the content has text for")
    args = parser.parse_args()
    known = texts()
    names = json.loads(Path(args.names).read_text())["names"] if args.names else sorted(known)
    table, basis = rows(names, None, known)
    have = sum(n in known for n in names)
    print(f"{len(names)} names, {have} with text, {len(basis.vocab)} words and pairs, {basis.dim} dims ({basis.model})")


if __name__ == "__main__":
    main()
