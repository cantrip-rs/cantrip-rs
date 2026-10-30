//! Worlds (docs/ai-player-plan.md, "Worlds"): one guess at the hidden game, consistent with all one seat knows, dealt
//! into a fork for a search to play on. Every card the seat cannot see and does not know is dealt again, out of what it
//! knows the deck to hold; every library card whose place it does not know is put somewhere else among the places it
//! does not know. Two games the seat cannot tell apart give it the same world for the same generator.
//!
//! The deal goes through `Game::rebind_hidden` and `GameState::reorder_library`, neither of them journalled: a
//! rollback inside the fork (a resolution prompt re-run) keeps the world. Then the question waiting is asked again
//! (`Game::ask_again`), of the world's cards and not the real ones.

use std::collections::{BTreeMap, HashMap, HashSet};

use cantrip_engine::visibility::{knows_face, looks_into, observable, request_objects};
use cantrip_engine::*;
use cantrip_obs::InfoMode;

use crate::knowledge::{Knowledge, front};

/// What decks the seat expects an opponent to play, in Hidden mode: the metagame's main decks, each a list of cards (by
/// their fronts) with a weight.
#[derive(Clone, Debug, Default)]
pub struct Prior {
    lists: Vec<(Vec<CardId>, f64)>,
}

impl Prior {
    pub fn new() -> Prior {
        Prior::default()
    }

    /// Adds a main deck, its cards by any face.
    pub fn add(&mut self, cards: &CardTable, list: impl IntoIterator<Item = CardId>, weight: f64) {
        let mut list: Vec<CardId> = list.into_iter().map(|c| cards.fronts.get(c.0 as usize).copied().flatten().unwrap_or(c)).collect();
        list.sort();
        if !list.is_empty() && weight > 0.0 {
            self.lists.push((list, weight));
        }
    }

    pub fn is_empty(&self) -> bool {
        self.lists.is_empty()
    }

    /// Every main deck in the decklists (`.txt`, "N Name" lines up to a "Sideboard" line) under `dir`, each weighted
    /// 1; a list naming a card this build does not have is left out.
    pub fn from_decks(dir: &std::path::Path) -> Result<Prior, String> {
        let mut paths = Vec::new();
        let mut dirs = vec![dir.to_path_buf()];
        while let Some(dir) = dirs.pop() {
            for entry in std::fs::read_dir(&dir).map_err(|e| format!("{}: {e}", dir.display()))?.filter_map(Result::ok) {
                let path = entry.path();
                if path.is_dir() {
                    dirs.push(path);
                } else if path.extension().is_some_and(|x| x == "txt") {
                    paths.push(path);
                }
            }
        }
        paths.sort();
        let mut prior = Prior::new();
        for path in paths {
            let text = std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
            let mut ids = Vec::new();
            let mut known = true;
            for line in text.lines().map(str::trim).take_while(|l| !l.eq_ignore_ascii_case("sideboard")) {
                if let Some((n, name)) = line.split_once(' ')
                    && let Ok(n) = n.parse::<usize>()
                {
                    match card(name.trim()) {
                        Some(id) => ids.extend(std::iter::repeat_n(id, n)),
                        None => known = false,
                    }
                }
            }
            if known {
                prior.add(cards(), ids, 1.0);
            }
        }
        if prior.is_empty() {
            return Err(format!("no decklist under {}", dir.display()));
        }
        Ok(prior)
    }
}

/// Where a card began the game: its owner's library, or outside the game (a sideboard).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum Origin {
    Library,
    Sideboard,
}

/// Deals a world for `seat` into `game`, a fork at its root (`Game::fork_for_search`, reseeded), from `k`, the seat's
/// knowledge of the real game (never refolded on a world: it reads the cards of the rows). In Full mode the world is
/// the game. In Decklist mode an opponent's unseen cards come out of their registered deck; in Hidden mode out of a
/// deck drawn from `prior` that holds every card the seat has seen them with (cards that began outside the game keep
/// what they are there: the prior has no sideboards).
pub fn determinize(game: &mut Game, k: &Knowledge, seat: PlayerId, mode: InfoMode, prior: &Prior, rng: &mut Rng) {
    determinize_owned(game, k, seat, mode, prior, rng, None);
}

/// `determinize`, dealing again only the cards `owner` owns where it names one: the rest of the world stays as it was
/// dealt. An information-set search deals each player's hidden cards with a generator of their own (`crate::issearch`),
/// so that worlds can share one player's deal and differ in the other's.
pub fn determinize_owned(game: &mut Game, k: &Knowledge, seat: PlayerId, mode: InfoMode, prior: &Prior, rng: &mut Rng, owner: Option<PlayerId>) {
    if mode == InfoMode::Full {
        return;
    }
    let dealt = |p: PlayerId| owner.is_none_or(|o| o == p);
    debug_assert_eq!(k.seat(), seat, "a world is dealt from the seat's own knowledge");
    // Another seat's companion, chosen at deck building, is theirs alone until it is revealed (CR 103.2b): a world neither
    // reveals it nor keeps it, and the card is a sideboard card like any other of theirs.
    for p in game.state.seats().collect::<Vec<_>>() {
        if p != seat && game.state.player(p).companion_to_reveal.is_some() {
            game.state.forget_companion_to_reveal(p);
        }
    }
    let s = &game.state;
    let players = s.players.len();
    let extra = match game.pending() {
        Some(r) if cantrip_engine::visibility::answering(game, r) == seat => request_objects(r),
        _ => HashSet::new(),
    };
    let visible = observable(game, seat, &extra);
    let sees = |id: ObjectId| visible.contains(&id) && knows_face(s, seat, id);
    // What each row became from, and the rows the seat was shown at some point.
    let mut from: HashMap<ObjectId, ObjectId> = HashMap::new();
    for (i, o) in s.objects.iter().enumerate() {
        if let Some(b) = o.became {
            from.insert(b, ObjectId(i as u32));
        }
    }
    let mut shown: HashSet<ObjectId> = HashSet::new();
    for record in &game.log {
        match record.event {
            Event::Revealed { card, .. } => {
                shown.insert(card);
            }
            Event::Looked { player, card } | Event::Shown { player, card } if player == seat => {
                shown.insert(card);
            }
            _ => {}
        }
    }
    // A row that is no longer the object (it became another) was seen if it was shown, or was where the seat looks.
    let row_seen = |id: ObjectId| {
        let zone = s.obj(id).zone;
        shown.contains(&id) || (looks_into(s, seat, zone) && knows_face(s, seat, id))
    };

    // The hidden objects, by zone slot then place in the zone (the unseen ones are dealt by their places once the
    // libraries are reordered).
    struct Unseen {
        owner: PlayerId,
        origin: Origin,
        chain: Vec<ObjectId>,
    }
    let mut unknown: Vec<Unseen> = Vec::new();
    let mut unplaced: Vec<ObjectId> = Vec::new();
    let mut accounted: BTreeMap<(PlayerId, Origin), Vec<CardId>> = BTreeMap::new();
    let mut rows: Vec<ObjectId> = Vec::new();
    for slot in 0..s.zones.len() {
        for &id in &s.zones[slot] {
            let o = s.obj(id);
            if o.token || o.card_copy || !live(s, id) {
                continue;
            }
            rows.push(id);
            let mut chain = vec![id];
            let mut root = id;
            let mut hidden = true;
            while let Some(&p) = from.get(&root) {
                root = p;
                hidden &= !row_seen(p);
                if hidden {
                    chain.push(p);
                }
            }
            let origin = match s.obj(root).zone.kind() {
                ZoneKind::Library => Origin::Library,
                ZoneKind::Sideboard => Origin::Sideboard,
                _ => continue,
            };
            let library = o.zone.is(ZoneKind::Library);
            let known = k.knows(id);
            // A row the object was before and the seat saw (a card of its hand shuffled back by a mulligan, a card
            // revealed and then put into its library): the object is that card, whatever the seat can still tell of
            // which object it is, as a card is the same card in every zone. Dealing it another would deal the seat a
            // history it did not see (and change its decklist, which counts the rows the deal began with).
            let seen_before = !hidden;
            if sees(id) || known.is_some() || seen_before {
                accounted.entry((o.owner, origin)).or_default().push(front(game, o.card));
                // Its place is dealt anew where the seat does not know it - but not where it sees it now (a library it is
                // searching shows its order).
                if library && !sees(id) && known != Some(true) && (known == Some(false) || seen_before) {
                    unplaced.push(id);
                }
                continue;
            }
            if !dealt(o.owner) {
                continue;
            }
            if library {
                unplaced.push(id);
            }
            unknown.push(Unseen { owner: o.owner, origin, chain });
        }
    }

    // What each owner's unseen cards are dealt from: the deck as dealt less what is accounted for.
    let mut pools: BTreeMap<(PlayerId, Origin), Vec<CardId>> = BTreeMap::new();
    for p in s.seats() {
        for origin in [Origin::Library, Origin::Sideboard] {
            let count = unknown.iter().filter(|u| u.owner == p && u.origin == origin).count();
            if count == 0 {
                continue;
            }
            let zone = match origin {
                Origin::Library => Zone::library(p),
                Origin::Sideboard => Zone::sideboard(p),
            };
            let dealt: Vec<CardId> = roots(s, zone).map(|id| front(game, s.obj(id).card)).collect();
            let mut list = if p == seat || mode == InfoMode::Decklist {
                dealt
            } else if origin == Origin::Library && !prior.is_empty() {
                let seen: BTreeMap<CardId, u16> = k.seen(p).collect();
                sample(prior, &seen, dealt.len(), rng)
            } else {
                // Cards that began outside the game, with no prior to say what they might be: they stay what they are.
                continue;
            };
            list.sort();
            let exact = p == seat || mode == InfoMode::Decklist;
            for card in accounted.get(&(p, origin)).into_iter().flatten() {
                if let Some(i) = list.iter().position(|c| c == card) {
                    list.remove(i);
                } else {
                    assert!(!(exact && verify()), "a card of seat {}'s seen in the game is not in their deck", p.0);
                }
            }
            if exact && verify() {
                assert_eq!(list.len(), count, "seat {}'s unseen cards are not what their deck less the cards accounted for holds", p.0);
            }
            fit(&mut list, count, rng);
            shuffle(&mut list, rng);
            pools.insert((p, origin), list);
        }
    }

    // The places, then the deal. An object's place is resampled among the library's unplaced places, so it is ordered
    // first by what the seat can tell objects apart by - the card for a known one, then when it went into the library
    // (its timestamp: every move takes a new one, a dealt card has 0) and the first event that names it - and only
    // then by its real place, which orders only objects nothing tells apart. The cards are dealt after, by the new
    // places, so which card lands where is the rng's alone.
    let mut changed = false;
    let unplaced: HashSet<ObjectId> = unplaced.into_iter().collect();
    for p in game.state.seats() {
        if !dealt(p) {
            continue;
        }
        let slot = Zone::library(p).slot(players);
        let zone = &game.state.zones[slot];
        // What a resolution in progress put into it keeps its place (`zone_order_fixed`): a rollback inside the fork takes
        // it away by place. What the resolution took from it and put back moves with the rest (`reorder_library`).
        let fixed = game.state.zone_order_fixed(slot);
        let at: Vec<usize> = (0..zone.len()).filter(|&i| unplaced.contains(&zone[i]) && !fixed.contains(&zone[i])).collect();
        if at.len() < 2 {
            continue;
        }
        let mut objects: Vec<ObjectId> = at.iter().map(|&i| zone[i]).collect();
        let s = &game.state;
        let named = |id: ObjectId| {
            if s.obj(id).timestamp != 0 {
                return 0;
            }
            game.log.iter().position(|r| r.event.is_about(id)).unwrap_or(usize::MAX)
        };
        let mut keyed: Vec<((u8, u16, u32, usize, usize), ObjectId)> = objects
            .iter()
            .enumerate()
            .map(|(place, &id)| {
                let card = match k.knows(id) {
                    Some(_) => (0, s.obj(id).card.0),
                    None => (1, 0),
                };
                ((card.0, card.1, s.obj(id).timestamp, named(id), place), id)
            })
            .collect();
        keyed.sort_unstable_by_key(|&(key, _)| key);
        objects = keyed.into_iter().map(|(_, id)| id).collect();
        shuffle(&mut objects, rng);
        let mut order = zone.clone();
        for (&i, id) in at.iter().zip(objects) {
            order[i] = id;
        }
        changed |= order != *zone;
        game.state.reorder_library(slot, order);
    }
    let place: HashMap<ObjectId, (usize, usize)> = game
        .state
        .zones
        .iter()
        .enumerate()
        .flat_map(|(slot, zone)| zone.iter().enumerate().map(move |(i, &id)| (id, (slot, i))))
        .collect();
    unknown.sort_by_key(|u| place[&u.chain[0]]);
    for u in &unknown {
        if let Some(pool) = pools.get_mut(&(u.owner, u.origin))
            && let Some(card) = pool.pop()
        {
            changed |= game.state.obj(u.chain[0]).card != card;
            game.rebind_hidden(&u.chain, card);
        }
    }
    // When an object arrived is the seat's to know only if an event records the arrival; one whose move no event
    // records (a card of an opening hand) takes the timestamp of a dealt card, so that its place in the deal is not
    // kept. That holds for one the seat sees now (a hand Duress shows) as much as for one it does not.
    let arrived: HashSet<ObjectId> = game
        .log
        .iter()
        .filter_map(|r| match r.event {
            Event::ZoneChange { new, .. } => Some(new),
            Event::Drew { card, .. } => Some(card),
            _ => None,
        })
        .collect();
    for id in rows {
        if game.state.obj(id).timestamp != 0 && !arrived.contains(&id) {
            game.state.restamp_hidden(id, 0);
            changed = true;
        }
    }
    // The question waiting was put to the real cards.
    if changed {
        game.ask_again();
    }
}

/// The cards dealt into a zone: its rows that no other row became, tokens and copies left out.
fn roots(s: &GameState, zone: Zone) -> impl Iterator<Item = ObjectId> + '_ {
    let became: HashSet<ObjectId> = s.objects.iter().filter_map(|o| o.became).collect();
    (0..s.objects.len() as u32).map(ObjectId).filter(move |&id| {
        let o = s.obj(id);
        o.zone == zone && !o.token && !o.card_copy && !became.contains(&id)
    })
}

/// A deck from the prior, of `size` cards, holding at least what the seat has seen (`seen`): one of the decks that
/// holds it all, by weight; when none does, the one that holds most of it, the rest put in for others at random.
fn sample(prior: &Prior, seen: &BTreeMap<CardId, u16>, size: usize, rng: &mut Rng) -> Vec<CardId> {
    let counts = |list: &[CardId]| {
        let mut m: BTreeMap<CardId, u16> = BTreeMap::new();
        for &c in list {
            *m.entry(c).or_default() += 1;
        }
        m
    };
    let held = |list: &[CardId]| {
        let m = counts(list);
        seen.iter().map(|(c, &n)| n.min(m.get(c).copied().unwrap_or(0)) as usize).sum::<usize>()
    };
    let total: usize = seen.values().map(|&n| n as usize).sum();
    let covering: Vec<usize> = (0..prior.lists.len()).filter(|&i| held(&prior.lists[i].0) == total).collect();
    let mut list = if !covering.is_empty() {
        let weights: Vec<f64> = covering.iter().map(|&i| prior.lists[i].1).collect();
        prior.lists[covering[pick(&weights, rng)]].0.clone()
    } else {
        // The best overlap, ties to the heavier and then the first.
        let best = (0..prior.lists.len())
            .max_by(|&a, &b| held(&prior.lists[a].0).cmp(&held(&prior.lists[b].0)).then(prior.lists[a].1.total_cmp(&prior.lists[b].1)).then(b.cmp(&a)))
            .expect("a prior with a deck");
        let mut list = prior.lists[best].0.clone();
        let have = counts(&list);
        let mut missing: Vec<CardId> = Vec::new();
        for (&c, &n) in seen {
            for _ in have.get(&c).copied().unwrap_or(0)..n {
                missing.push(c);
            }
        }
        // Drop as many cards as are missing, none of them one of the seen, then put the missing in.
        let mut keep = counts(&[]);
        for (&c, &n) in seen {
            keep.insert(c, n);
        }
        for _ in 0..missing.len() {
            let spare: Vec<usize> = (0..list.len()).filter(|&i| counts(&list).get(&list[i]).copied().unwrap_or(0) > keep.get(&list[i]).copied().unwrap_or(0)).collect();
            if spare.is_empty() {
                break;
            }
            list.remove(spare[rng.below(spare.len() as u32) as usize]);
        }
        list.extend(missing);
        list
    };
    list.sort();
    // To the size of the deck dealt: the cards the seat saw are never the ones dropped.
    while list.len() > size {
        let m = counts(&list);
        let spare: Vec<usize> = (0..list.len()).filter(|&i| m[&list[i]] > seen.get(&list[i]).copied().unwrap_or(0)).collect();
        if spare.is_empty() {
            break;
        }
        list.remove(spare[rng.below(spare.len() as u32) as usize]);
    }
    fit(&mut list, size, rng);
    list.sort();
    list
}

/// Pads a list with copies of its own cards, or trims it, at random, to `size`.
fn fit(list: &mut Vec<CardId>, size: usize, rng: &mut Rng) {
    while list.len() > size {
        list.remove(rng.below(list.len() as u32) as usize);
    }
    while list.len() < size && !list.is_empty() {
        let c = list[rng.below(list.len() as u32) as usize];
        list.push(c);
    }
}

fn pick(weights: &[f64], rng: &mut Rng) -> usize {
    let total: f64 = weights.iter().sum();
    let mut x = (rng.next_u32() as f64 / (u32::MAX as f64 + 1.0)) * total;
    for (i, &w) in weights.iter().enumerate() {
        if x < w {
            return i;
        }
        x -= w;
    }
    weights.len() - 1
}

/// Fisher-Yates.
fn shuffle<T>(items: &mut [T], rng: &mut Rng) {
    for i in (1..items.len()).rev() {
        let j = rng.below(i as u32 + 1) as usize;
        items.swap(i, j);
    }
}

fn verify() -> bool {
    static VERIFY: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *VERIFY.get_or_init(|| std::env::var_os("CANTRIP_VERIFY").is_some_and(|v| v != "0"))
}
