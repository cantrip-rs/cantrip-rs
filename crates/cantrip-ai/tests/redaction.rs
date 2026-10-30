//! Redaction: two games that differ only in what a seat cannot know give it the same observation, byte for byte.
//!
//! Each pair is a game A and a game B dealt alike but for the seat's opponent's library: a card of the opening hand
//! traded with a deep one, the deep part of both libraries put in another order (and in hidden mode one deep card of
//! the opponent's another card altogether; in decklist mode the opponent's decklist is the same). B is played in
//! lockstep with A, each of A's answers turned into B's by the seat's canonical index. While a witness of what the
//! seat can tell - the cards it sees now, and every card it was shown or saw move, drawn from the game directly and
//! not from the observation or the knowledge - is the same in both, the two observations must be equal; once it
//! differs the pair stops. So a fault in the knowledge shows too.
//!
//! Where the observations are compared, each game is also forked and a world dealt into it for the seat
//! (`cantrip_ai::determinize`), with the same seeds in both: the two worlds must be the same game - every seat's Full
//! observation, and every live object, the stack and the history with their ids numbered canonically. Now and then
//! both worlds play on for 40 decisions, chosen in world A alone and turned into B's, and must stay the same.
//!
//! The fast test plays a few bot games and hidden-zone scenarios; `CANTRIP_REDACTION_GAMES=n` plays n bot games and
//! every scenario that looks into a hidden zone (`cargo test -p cantrip-ai --test redaction -- --ignored`). With
//! `DUMP_WORLDS=dir`, two worlds that differ are written there (`ca.txt`, `cb.txt`, and the observations).

use std::collections::{BTreeSet, HashMap};
use std::path::{Path, PathBuf};

use cantrip_ai::{Knowledge, Prior, determinize, observe_indexed};
use cantrip_bench::Random;
use cantrip_engine::visibility::{knows_face, looks_into, observable, request_objects};
use cantrip_engine::*;
use cantrip_obs::encode;
use cantrip_obs::factor::Machine;
use cantrip_obs::{InfoMode, ObjectObs, Observation};
use cantrip_script::GameScript;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// A game to play twice: its setup, and how it is played.
struct Case {
    name: String,
    decks: Vec<Vec<CardId>>,
    sideboards: Vec<Vec<CardId>>,
    basic_lands_outside: Vec<CardId>,
    chooser: Option<PlayerId>,
    dice: Vec<u8>,
    life: Vec<(usize, i32)>,
    event_limit: Option<usize>,
    /// A scenario's answers in order; none for a bot game, which the bench's random seat plays.
    answers: Option<Vec<(PlayerId, Answer)>>,
    bot: u64,
    /// How deep a library is put in another order.
    deep: usize,
}

fn ids(names: &[String]) -> Vec<CardId> {
    names.iter().map(|n| card(n).map(|id| cards().fronts[id.0 as usize].unwrap_or(id)).unwrap_or_else(|| panic!("no card {n}"))).collect()
}

fn bot_case(i: u64) -> Case {
    let decks = [root().join("decks/spike/spike-rg.txt"), root().join("decks/spike/spike-ubw.txt")];
    let mut random = Random::new(1000 + i);
    let mut decks: Vec<Vec<CardId>> = decks.iter().map(|d| ids(&cantrip_bench::load_deck(d))).collect();
    if i % 2 == 1 {
        decks.swap(0, 1);
    }
    for deck in &mut decks {
        for k in (1..deck.len()).rev() {
            deck.swap(k, random.below(k + 1));
        }
    }
    Case {
        name: format!("bot game {i}"),
        decks,
        sideboards: vec![],
        basic_lands_outside: vec![],
        chooser: None,
        dice: vec![],
        life: vec![],
        event_limit: None,
        answers: None,
        bot: i,
        deep: 15,
    }
}

fn scenario_case(path: &Path) -> Option<Case> {
    let script = GameScript::load(path).ok()?;
    let result = cantrip_script::run(&script).ok()?;
    let game = result.game;
    let answers = game.tree.path(game.at).into_iter().filter_map(|node| game.tree.answer(node).map(|(_, p, a)| (*p, a.clone()))).collect();
    let decks = (0..script.decks.len()).map(|i| ids(&script.decklist(i))).collect();
    let sideboards = (0..script.sideboards.len()).map(|i| ids(&script.sideboard(i))).collect();
    Some(Case {
        name: path.file_name()?.to_string_lossy().into_owned(),
        decks,
        sideboards,
        basic_lands_outside: ids(&script.basic_lands_outside),
        chooser: script.opening.then(|| PlayerId((script.chooser.max(1) - 1) as u8)),
        dice: script.dice.clone(),
        life: script.life.clone(),
        event_limit: script.event_limit,
        answers: Some(answers),
        bot: 0,
        deep: 3,
    })
}

/// Deals the case; `variant` makes game B for `seat` in `mode`, `None` game A.
fn deal(case: &Case, variant: Option<(PlayerId, InfoMode, u64)>) -> Game {
    let mut decks = case.decks.clone();
    let mut random = Random::new(variant.map_or(0, |(seat, _, salt)| salt * 2 + seat.0 as u64 + 7));
    if let Some((seat, mode, _)) = variant {
        let them = 1 - seat.0 as usize;
        let len = decks[them].len();
        if len > 7 {
            // A deep card unlike the one it trades with, where there is one.
            let held = random.below(7);
            let from = if len > 20 { 20 } else { len - 1 };
            let unlike: Vec<usize> = (from..len).filter(|&k| decks[them][k] != decks[them][held]).collect();
            let deep = if unlike.is_empty() { from + random.below(len - from) } else { unlike[random.below(unlike.len())] };
            decks[them].swap(held, deep);
            if mode == InfoMode::Hidden {
                let other = decks[seat.0 as usize].iter().copied().find(|&c| c != decks[them][deep]);
                if let Some(other) = other {
                    decks[them][deep] = other;
                }
            }
        }
    }
    let setup = Setup {
        decks,
        sideboards: case.sideboards.clone(),
        basic_lands_outside: case.basic_lands_outside.clone(),
        companions: Vec::new(),
        minimum_deck_size: 60,
        // Another generator for B: a shuffle the seat cannot see comes out otherwise.
        seed: if variant.is_some() { 2 } else { 1 },
        shuffle: false,
        opening_chooser: case.chooser,
    };
    let mut game = Game::new(cards(), &setup);
    if variant.is_some() {
        let n = game.state.players.len();
        for p in game.state.seats().collect::<Vec<_>>() {
            let library = &mut game.state.zones[Zone::library(p).slot(n)];
            if library.len() > case.deep + 1 {
                let tail = &mut library[case.deep..];
                for k in (1..tail.len()).rev() {
                    tail.swap(k, random.below(k + 1));
                }
            }
        }
    }
    game.verify = std::env::var_os("CANTRIP_VERIFY").is_some();
    game.run();
    game.state.force_rolls(&case.dice);
    for &(seat, life) in &case.life {
        if let Some(player) = game.state.players.get_mut(seat) {
            player.life = life;
        }
    }
    if let Some(limit) = case.event_limit {
        game.max_events = limit;
    }
    game
}

/// A's answers in order: the scenario's, or the bot's as it plays A.
fn answers(case: &Case) -> Vec<(PlayerId, Answer)> {
    if let Some(answers) = &case.answers {
        return answers.clone();
    }
    let mut game = deal(case, None);
    let mut random = Random::new(case.bot);
    let mut out = Vec::new();
    while let Some(request) = game.pending().cloned() {
        if game.is_over() || out.len() >= 1500 {
            break;
        }
        let answer = cantrip_bench::decide(&game, &mut random, &request);
        game.answer(request.id, request.player, answer.clone()).expect("the bot answers legally");
        out.push((request.player, answer));
    }
    out
}

/// A short, id-free description of an object as the seat may tell it.
fn short(game: &Game, seat: PlayerId, id: ObjectId) -> String {
    if id.0 & 0x8000_0000 != 0 {
        return format!("ability {}", id.0 & 0x7fff_ffff);
    }
    let o = game.state.obj(id);
    let name = if knows_face(&game.state, seat, id) { game.name(id) } else { "?" };
    format!("{name}@{:?}{}", o.zone.kind(), o.zone.player.0)
}

fn target(game: &Game, seat: PlayerId, t: &Target) -> String {
    match *t {
        Target::Object(id) => short(game, seat, id),
        other => format!("{other:?}"),
    }
}

fn cut(debug: String) -> String {
    debug.split(['{', '(', ' ']).next().unwrap_or("").to_string()
}

/// What the seat may tell of the game as it stands, from the game alone (neither the observation nor the knowledge):
/// with the same trail, two games with the same witness must give the same observation.
fn witness(game: &Game, seat: PlayerId) -> Vec<String> {
    let s = &game.state;
    let request = game.pending();
    let extra = match request {
        Some(r) if cantrip_engine::visibility::answering(game, r) == seat => request_objects(r),
        _ => Default::default(),
    };
    let visible = observable(game, seat, &extra);
    let mut lines = BTreeSet::new();
    for &id in visible.iter().filter(|id| id.0 & 0x8000_0000 == 0) {
        let o = s.obj(id);
        let ordered = matches!(o.zone.kind(), ZoneKind::Library | ZoneKind::Graveyard);
        let position = if ordered { s.zone(o.zone).iter().position(|&x| x == id) } else { None };
        let attacking = s.attackers.iter().find(|a| a.id == id).map(|a| (target(game, seat, &a.target), a.blocked));
        let blocking: Vec<String> = s.blocks.iter().filter(|b| b.blocker == id).map(|b| short(game, seat, b.attacker)).collect();
        let attached = o.attached_to.map(|a| short(game, seat, a));
        let face = knows_face(s, seat, id);
        let chars = if face || o.zone.is(ZoneKind::Battlefield) { format!("{:?}", game.chars(id)) } else { String::new() };
        let counters: Vec<(usize, i16)> = o.counter_array().iter().enumerate().filter(|&(_, &n)| n != 0).map(|(k, &n)| (k, n)).collect();
        lines.insert(format!(
            "obj {} {position:?} own{} ctl{} tap{} dmg{} {counters:?} {attacking:?} {blocking:?} {attached:?} {:?} fd{} tok{} {chars}",
            short(game, seat, id),
            o.owner.0,
            o.controller.0,
            o.tapped,
            o.damage,
            o.attached_player,
            o.face_down,
            o.token,
        ));
    }
    let mut out: Vec<String> = lines.into_iter().collect();
    for (i, item) in s.stack.iter().enumerate() {
        let targets: Vec<String> = game.state.targets(item).iter().map(|t| target(game, seat, t)).collect();
        out.push(format!("stack {i} {:?} {} ctl{} {targets:?} x{}", item.kind, short(game, seat, item.object), item.controller.0, item.x));
    }
    for p in s.seats() {
        let pl = &s.players[p.0 as usize];
        let counts: Vec<usize> = [Zone::library(p), Zone::hand(p), Zone::graveyard(p), Zone::exile(p), Zone::sideboard(p)].iter().map(|&z| s.zone(z).len()).collect();
        out.push(format!(
            "player {} {} {} {} {} {} {} {:?} {} {:?} {counts:?}",
            p.0, pl.life, pl.poison(), pl.energy(), pl.lost, pl.lands_played, pl.mulligans, pl.pool, pl.monarch, pl.speed
        ));
    }
    let t = &s.turn;
    out.push(format!("turn {} {} {} {} {}", t.number, t.active.0, t.step_raw, t.priority.0, t.starting.0));
    if let Some(r) = request {
        let mut candidates: Vec<String> = if r.player == seat { extra.iter().map(|&id| short(game, seat, id)).collect() } else { vec![] };
        candidates.sort();
        out.push(format!("request {} {} {candidates:?}", r.player.0, cut(format!("{:?}", r.kind))));
    }
    let events: Vec<String> = game.log.iter().rev().take(64).map(|r| format!("{}:{}", r.turn, cut(format!("{:?}", r.event)))).collect();
    out.push(format!("events {events:?}"));
    out
}

/// Every card the seat was shown or saw move, record by record, by name: what its knowledge is folded from, kept
/// apart from the knowledge so that a fault in the fold shows as a difference in the observations.
#[derive(Default)]
struct Trail(Vec<String>, usize, Option<Record>);

impl Trail {
    fn extend(&mut self, game: &Game, seat: PlayerId) {
        let name = |id: ObjectId| game.cards.get(game.state.obj(id).card).name;
        // A history cut back (a spell given up while announced) is followed again from the start.
        if self.1 > game.log.len() || (self.1 > 0 && game.log.get(self.1 - 1) != self.2.as_ref()) {
            *self = Trail::default();
        }
        for record in &game.log[self.1..] {
            let line = match record.event {
                Event::Revealed { card, .. } => format!("revealed {}", name(card)),
                Event::Looked { player, card } | Event::Shown { player, card } if player == seat => format!("shown {}", name(card)),
                Event::Looked { player, .. } | Event::Shown { player, .. } => format!("shown to {}", player.0),
                Event::Shuffled { player } => format!("shuffled {}", player.0),
                Event::Randomized { player, .. } => format!("randomized {}", player.0),
                Event::TurnedFaceUp { permanent, .. } => format!("face up {}", name(permanent)),
                Event::ZoneChange { old, new, from, to, .. } => {
                    let sees = |zone: Zone| looks_into(&game.state, seat, zone) && !zone.is(ZoneKind::Gone);
                    let what = if (sees(from) && knows_face(&game.state, seat, old)) || (sees(to) && knows_face(&game.state, seat, new)) { name(new) } else { "?" };
                    format!("moved {what} {:?}{} {:?}{}", from.kind(), from.player.0, to.kind(), to.player.0)
                }
                _ => continue,
            };
            self.0.push(line);
        }
        self.1 = game.log.len();
        self.2 = game.log.last().copied();
    }
}

/// Turns an object of A into the one of B the seat cannot tell from it: by canonical index where the seat sees it,
/// else by its place in its zone; with `rotate`, a card in a zone hidden from the seat by the next place in it, so
/// that another player's hidden choice in B is another card than in A.
fn mapper<'a>(
    a: &'a Game,
    b: &'a Game,
    seat: PlayerId,
    rotate: bool,
    indexed: Option<(&'a [ObjectId], &'a Observation, &'a [ObjectId])>,
) -> impl Fn(ObjectId) -> Option<ObjectId> + 'a {
    move |id| {
        if let Some((ia, obs, ib)) = indexed
            && let Some(i) = ia.iter().position(|&x| x == id)
            && !matches!(obs.objects[i], ObjectObs::Hidden { .. })
        {
            return ib.get(i).copied();
        }
        if id.0 & 0x8000_0000 != 0 {
            return None;
        }
        let zone = a.state.obj(id).zone;
        let at = a.state.zone(zone).iter().position(|&x| x == id)?;
        let there = b.state.zone(zone);
        let hidden = zone.kind().is_hidden() && !looks_into(&a.state, seat, zone);
        let at = if rotate && hidden && !there.is_empty() { (at + 1) % there.len() } else { at };
        there.get(at).copied()
    }
}

fn map_answer(answer: &Answer, f: &dyn Fn(ObjectId) -> Option<ObjectId>) -> Option<Answer> {
    let t = |t: &Target| -> Option<Target> {
        Some(match *t {
            Target::Object(id) => Target::Object(f(id)?),
            other => other,
        })
    };
    let ts = |v: &[Target]| v.iter().map(t).collect::<Option<Vec<_>>>();
    let os = |v: &[ObjectId]| v.iter().map(|&id| f(id)).collect::<Option<Vec<_>>>();
    Some(match answer {
        Answer::Act(a) => {
            let mut a = a.clone();
            a.source = match a.source {
                Some(id) => Some(f(id)?),
                None => None,
            };
            a.targets = ts(&a.targets)?;
            Answer::Act(a)
        }
        Answer::Cards(v) => Answer::Cards(os(v)?),
        Answer::Targets(v) => Answer::Targets(ts(v)?),
        Answer::Attack(v) => Answer::Attack(v.iter().map(|(id, tg)| Some((f(*id)?, t(tg)?))).collect::<Option<_>>()?),
        Answer::Block(v) => Answer::Block(v.iter().map(|(x, y)| Some((f(*x)?, f(*y)?))).collect::<Option<_>>()?),
        Answer::Shares(v) => Answer::Shares(
            v.iter()
                .map(|(x, y, n)| {
                    let y = match y {
                        Some(y) => Some(f(*y)?),
                        None => None,
                    };
                    Some((f(*x)?, y, *n))
                })
                .collect::<Option<_>>()?,
        ),
        Answer::Placements { placements, shuffle } => {
            Answer::Placements { placements: placements.iter().map(|p| os(p)).collect::<Option<_>>()?, shuffle: *shuffle }
        }
        Answer::Divided(v) => Answer::Divided(v.iter().map(|(tg, n)| Some((t(tg)?, *n))).collect::<Option<_>>()?),
        Answer::Counters(v) => Answer::Counters(v.iter().map(|(id, k, n)| Some((f(*id)?, *k, *n))).collect::<Option<_>>()?),
        other => other.clone(),
    })
}

#[derive(Default)]
struct Tally {
    compared: usize,
    pairs: usize,
    worlds: usize,
    stand_ins: usize,
    /// Comparisons where A's and B's histories already name hidden objects otherwise, so no world is dealt.
    apart: usize,
}

/// The metagame for Hidden mode's worlds: every main deck under `decks/` whose cards are all in the engine, alike.
fn prior() -> &'static Prior {
    static PRIOR: std::sync::OnceLock<Prior> = std::sync::OnceLock::new();
    PRIOR.get_or_init(|| {
        let mut prior = Prior::new();
        let mut paths = Vec::new();
        let mut dirs = vec![root().join("decks")];
        while let Some(dir) = dirs.pop() {
            for entry in std::fs::read_dir(&dir).expect("decks").filter_map(Result::ok) {
                let path = entry.path();
                if path.is_dir() {
                    dirs.push(path);
                } else if path.extension().is_some_and(|x| x == "txt") {
                    paths.push(path);
                }
            }
        }
        paths.sort();
        for path in paths {
            // The main deck: "N Name" lines, up to a "Sideboard" line.
            let text = std::fs::read_to_string(&path).expect("a decklist");
            let mut names = Vec::new();
            for line in text.lines().map(str::trim).take_while(|l| !l.eq_ignore_ascii_case("sideboard")) {
                if let Some((n, name)) = line.split_once(' ')
                    && let Ok(n) = n.parse::<usize>()
                {
                    names.extend(std::iter::repeat_n(name.trim().to_string(), n));
                }
            }
            if !names.is_empty() && names.iter().all(|n| card(n).is_some()) {
                prior.add(cards(), ids(&names), 1.0);
            }
        }
        assert!(!prior.is_empty(), "a metagame to deal from");
        prior
    })
}

/// A seat's Full observation of a world, with its index.
fn full(world: &Game, p: PlayerId) -> Option<(Observation, Vec<ObjectId>)> {
    observe_indexed(world, &Knowledge::new(p), p, InfoMode::Full)
}

/// A game as text with no engine ids in it: its history, every live object by zone and place, and the stack, each
/// `ObjectId` numbered by where it first appears. The history's references to the rows an object was before
/// (`ZoneChange { old, .. }`) come first, so the `became` chains are numbered through too.
fn canon(world: &Game) -> Vec<String> {
    canon_of(world, true)
}

/// The history alone, or with the objects and the stack (`canon`).
fn canon_of(world: &Game, objects: bool) -> Vec<String> {
    let mut numbers: HashMap<u32, usize> = HashMap::new();
    let mut renumber = |text: String| -> String {
        let mut out = String::with_capacity(text.len());
        let mut rest = text.as_str();
        while let Some(i) = rest.find("ObjectId(") {
            out.push_str(&rest[..i]);
            let after = &rest[i + "ObjectId(".len()..];
            let end = after.find(')').expect("an id");
            let n: u32 = after[..end].parse().expect("a number");
            let next = numbers.len();
            out += &format!("#{}", *numbers.entry(n).or_insert(next));
            rest = &after[end + 1..];
        }
        out.push_str(rest);
        out
    };
    let s = &world.state;
    let mut out: Vec<String> = world.log.iter().map(|r| renumber(format!("{r:?}"))).collect();
    if !objects {
        return out;
    }
    for (slot, zone) in s.zones.iter().enumerate() {
        for (at, &id) in zone.iter().enumerate() {
            if live(s, id) {
                out.push(renumber(format!("{slot} {at} {id:?} {:?}", s.obj(id))));
            }
        }
    }
    out.push(renumber(format!("{:?} {:?}", s.stack, s.triggers)));
    out
}

/// The two worlds are the same game.
fn same_worlds(name: &str, step: usize, wa: &Game, wb: &Game) {
    for p in wa.state.seats() {
        let (oa, ob) = (full(wa, p), full(wb, p));
        assert_eq!(oa.is_some(), ob.is_some(), "{name}: step {step}: one world's observation for seat {} deferred", p.0);
        if let (Some((oa, _)), Some((ob, _))) = (&oa, &ob)
            && cantrip_obs::to_bytes(oa) != cantrip_obs::to_bytes(ob)
        {
            if let Some(dir) = std::env::var_os("DUMP_WORLDS") {
                let dir = PathBuf::from(dir);
                std::fs::write(dir.join("oa.txt"), format!("{oa:#?}")).ok();
                std::fs::write(dir.join("ob.txt"), format!("{ob:#?}")).ok();
                std::fs::write(dir.join("ca.txt"), canon(wa).join("\n")).ok();
                std::fs::write(dir.join("cb.txt"), canon(wb).join("\n")).ok();
            }
            panic!("{}", leak(&format!("{name} (worlds)"), p, InfoMode::Full, step, oa, ob));
        }
    }
    let (ca, cb) = (canon(wa), canon(wb));
    if ca != cb {
        if let Some(dir) = std::env::var_os("DUMP_WORLDS") {
            let dir = PathBuf::from(dir);
            std::fs::write(dir.join("ca.txt"), ca.join("\n")).ok();
            std::fs::write(dir.join("cb.txt"), cb.join("\n")).ok();
        }
        let mut out = format!("{name}: step {step}: the worlds differ, ids numbered canonically\n");
        for (x, y) in ca.iter().zip(&cb).filter(|(x, y)| x != y).take(6) {
            out += &format!("  A {x}\n  B {y}\n");
        }
        if ca.len() != cb.len() {
            out += &format!("  (and they differ in length: {} and {})\n", ca.len(), cb.len());
        }
        panic!("{out}");
    }
}

/// Forks A and B, deals each a world for `seat` from its own knowledge with the same seeds, and checks they are the
/// same game; with `stand_in`, plays both on for 40 decisions (a search's playouts, standing in for one), each chosen
/// by the bench's random seat in world A alone and turned into B's by the requesting seat's Full index.
#[allow(clippy::too_many_arguments)]
fn worlds(name: &str, step: usize, a: &Game, b: &Game, ka: &Knowledge, kb: &Knowledge, seat: PlayerId, mode: InfoMode, stand_in: bool) {
    let seed = 0x5eed + step as u64;
    let deal = |game: &Game, k: &Knowledge| {
        let mut world = game.fork_for_search();
        world.reseed(seed);
        determinize(&mut world, k, seat, mode, prior(), &mut Rng::new(seed));
        world
    };
    let (mut wa, mut wb) = (deal(a, ka), deal(b, kb));
    let name = format!("{name}: seat {} {mode:?}", seat.0);
    same_worlds(&name, step, &wa, &wb);
    if !stand_in {
        return;
    }
    let mut random = Random::new(seed);
    for played in 0..40 {
        let (ra, rb) = (wa.pending().cloned(), wb.pending().cloned());
        assert_eq!(ra.is_some(), rb.is_some(), "{name}: step {step}+{played}: one world asks, the other does not");
        let (Some(ra), Some(rb)) = (ra, rb) else { break };
        if wa.is_over() {
            break;
        }
        assert_eq!(ra.player, rb.player, "{name}: step {step}+{played}: the worlds ask different players");
        let answer = cantrip_bench::decide(&wa, &mut random, &ra);
        let mapped = {
            let (ia, ib) = (full(&wa, ra.player), full(&wb, ra.player));
            let indexed = match (&ia, &ib) {
                (Some((obs, ia)), Some((_, ib))) => Some((ia.as_slice(), obs, ib.as_slice())),
                _ => None,
            };
            map_answer(&answer, &mapper(&wa, &wb, ra.player, false, indexed)).expect("world B has every object world A has")
        };
        if let Err(e) = wa.answer(ra.id, ra.player, answer.clone()) {
            let source = match ra.kind {
                RequestKind::ChooseCards { source, .. } => wa.cards.get(wa.state.obj(source).card).name,
                _ => "",
            };
            let board: Vec<&str> = wa.state.battlefield().iter().filter(|&&id| wa.state.obj(id).controller == ra.player).map(|&id| wa.cards.get(wa.state.obj(id).card).name).collect();
            panic!("{name}: step {step}+{played}: the bot answers world A legally: {e:?}\n  request {ra:?} (source {source})\n  answer {answer:?}\n  battlefield {board:?}\n  last events {:?}", &wa.log[wa.log.len().saturating_sub(8)..].iter().map(|r| &r.event).collect::<Vec<_>>());
        }
        wb.answer(rb.id, rb.player, mapped).expect("world B takes world A's answer");
        same_worlds(&name, step + played + 1, &wa, &wb);
    }
}

/// Plays A and B in lockstep for `seat` in `mode`, comparing while the witnesses agree.
fn pair(case: &Case, answers: &[(PlayerId, Answer)], seat: PlayerId, mode: InfoMode, rotate: bool, tally: &mut Tally) {
    let mut a = deal(case, None);
    let mut b = deal(case, Some((seat, mode, case.bot)));
    let mut ka = Knowledge::new(seat);
    let mut kb = Knowledge::new(seat);
    let (mut ta, mut tb) = (Trail::default(), Trail::default());
    tally.pairs += 1;
    for (step, (player, answer)) in answers.iter().enumerate() {
        ka.update(&a);
        kb.update(&b);
        ta.extend(&a, seat);
        tb.extend(&b, seat);
        if ta.0 != tb.0 || witness(&a, seat) != witness(&b, seat) {
            break;
        }
        let oa = observe_indexed(&a, &ka, seat, mode);
        let ob = observe_indexed(&b, &kb, seat, mode);
        assert_eq!(oa.is_some(), ob.is_some(), "{}: seat {} {mode:?} step {step}: one observation deferred", case.name, seat.0);
        if let (Some((oa, _)), Some((ob, _))) = (&oa, &ob) {
            tally.compared += 1;
            if cantrip_obs::to_bytes(oa) != cantrip_obs::to_bytes(ob) {
                panic!("{}", leak(&format!("{}{}", case.name, if rotate { " (rotated)" } else { "" }), seat, mode, step, oa, ob));
            }
            // And so do their encodings, at the request's first step: what the network reads cannot tell them apart.
            let step_of = |o: &Observation| o.request.as_ref().and_then(|r| r.ask.as_ref()).and_then(|ask| Machine::new(ask, &[]).step().cloned()).unwrap_or_default();
            let hashed = |name: &str| 1 + (encode::fnv1a64(name.as_bytes()) % 4096) as u32;
            let (ea, eb) = (encode::encode(oa, &step_of(oa), &[], &[], &hashed), encode::encode(ob, &step_of(ob), &[], &[], &hashed));
            assert!(encode::to_compact(&ea) == encode::to_compact(&eb), "{}: seat {} {mode:?} step {step}: equal observations encode differently", case.name, seat.0);
            // A world now and then, and now and then one played on: every pair is dealt at least one.
            // A and B may differ in which of the seat's unseen objects an event named (the opponent discarded the card
            // it just drew in A, one it had kept in B): no world resamples that (docs/known-issues.md), so none is
            // dealt there.
            if (tally.compared % WORLDS_EVERY == 0 || step == 0) && canon_of(&a, false) != canon_of(&b, false) {
                tally.apart += 1;
            } else if tally.compared % WORLDS_EVERY == 0 || step == 0 {
                let stand_in = tally.worlds % STAND_IN_EVERY == 0;
                worlds(&case.name, step, &a, &b, &ka, &kb, seat, mode, stand_in);
                tally.worlds += 1;
                tally.stand_ins += stand_in as usize;
            }
        }
        let (Some(ra), Some(rb)) = (a.pending().cloned(), b.pending().cloned()) else { break };
        assert_eq!(ra.player, *player, "{}: the recorded answer is for the player asked", case.name);
        let indexed = match (&oa, &ob) {
            (Some((obs, ia)), Some((_, ib))) => Some((ia.as_slice(), obs, ib.as_slice())),
            _ => None,
        };
        let Some(mapped) = map_answer(answer, &mapper(&a, &b, seat, rotate, indexed)) else { break };
        a.answer(ra.id, ra.player, answer.clone()).expect("A replays its own answers");
        if b.answer(rb.id, rb.player, mapped).is_err() {
            break;
        }
    }
    // The fold record by record agrees with a fold from scratch.
    ka.update(&a);
    assert_eq!(ka, Knowledge::of(&a, seat), "{}: seat {}'s knowledge folded as the game went differs from a fold from scratch", case.name, seat.0);
}

fn leak(name: &str, seat: PlayerId, mode: InfoMode, step: usize, a: &Observation, b: &Observation) -> String {
    let mut out = format!("{name}: seat {} {mode:?} step {step}: the observations differ though the seat cannot tell the games apart\n", seat.0);
    let da = format!("{a:#?}");
    let db = format!("{b:#?}");
    for (i, (x, y)) in da.lines().zip(db.lines()).enumerate() {
        if x != y {
            out += &format!("  line {i}: A {x}\n          B {y}\n");
        }
    }
    if da.lines().count() != db.lines().count() {
        out += "  (and they differ in length)\n";
    }
    out
}

/// How often a world is dealt, in observations compared; how often one is played on, in worlds dealt.
const WORLDS_EVERY: usize = 13;
const STAND_IN_EVERY: usize = 4;

fn check(cases: impl Iterator<Item = Case>) -> Tally {
    let mut tally = Tally::default();
    for case in cases {
        let answers = answers(&case);
        for seat in [PlayerId(0), PlayerId(1)] {
            for mode in [InfoMode::Hidden, InfoMode::Decklist] {
                for rotate in [false, true] {
                    pair(&case, &answers, seat, mode, rotate, &mut tally);
                }
            }
        }
    }
    tally
}

const SCENARIOS: &[&str] = &[
    "a-hand-seen-by-duress-hands-over-with-its-cards-known",
    "a-hand-seen-by-duress-hands-over-to-the-player-who-saw-it",
    "accumulate-wisdom-puts-one-of-the-top-three-into-your-hand-and-the-rest-on-the-bottom",
    "accumulate-wisdom-with-a-library-of-two-looks-at-both-and-takes-one",
    "aminatou-surveils-two-at-the-beginning-of-her-controllers-upkeep",
    "analyze-the-pollen-without-evidence-finds-only-a-basic-land-and-reveals-it",
    "a-land-fetched-onto-the-battlefield-asks-its-type-from-the-library",
    "a-mulligan-to-six-puts-the-chosen-card-on-the-bottom",
    // The opponent's hand shown to the seat: the cards it knows came in with the opening hand, whose draws no event
    // records, so their timestamps must not keep the real deal order.
    "ancient-vendetta-leaves-the-named-permanent-on-the-battlefield-alone",
    "blightsteel-colossus-countered-is-shuffled-into-the-library-instead-of-the-graveyard",
    // A hand revealed by Duress after the opponent drew and played an Island: which Island they played, the drawn one
    // or another, the seat cannot tell, so the draw's event must not name a card it sees in the hand now.
    "mox-jet-taps-once-per-untap-for-black-and-the-turn-it-is-cast",
];

#[test]
fn games_the_seat_cannot_tell_apart_look_the_same() {
    let scenarios = SCENARIOS.iter().map(|n| scenario_case(&root().join("scenarios").join(format!("{n}.cantrip"))).unwrap_or_else(|| panic!("{n} plays")));
    let tally = check((0..12).map(bot_case).chain(scenarios));
    eprintln!("{} observations compared over {} pairs; {} worlds dealt, {} played on, {} not dealt for histories apart", tally.compared, tally.pairs, tally.worlds, tally.stand_ins, tally.apart);
    assert!(tally.compared >= 2000, "only {} observations compared over {} pairs", tally.compared, tally.pairs);
    assert!(tally.stand_ins >= 40, "only {} worlds played on", tally.stand_ins);
}

#[test]
#[ignore = "minutes: CANTRIP_REDACTION_GAMES bot games (default 300) and every scenario that looks into a hidden zone"]
fn many_games_the_seat_cannot_tell_apart_look_the_same() {
    let games: u64 = std::env::var("CANTRIP_REDACTION_GAMES").ok().and_then(|v| v.parse().ok()).unwrap_or(300);
    let words = ["Duress", "Surveil", "Scry", "Look", "look", "Reveal", "reveal", "Search", "search", "Mill", "Shuffle", "shuffle", "face down", "Thoughtseize"];
    let mut paths: Vec<PathBuf> = std::fs::read_dir(root().join("scenarios"))
        .expect("scenarios")
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "cantrip"))
        .filter(|p| std::fs::read_to_string(p).is_ok_and(|t| words.iter().any(|w| t.contains(w))))
        .collect();
    paths.sort();
    let tally = check((0..games).map(bot_case).chain(paths.iter().filter_map(|p| scenario_case(p))));
    eprintln!("{} observations compared over {} pairs; {} worlds dealt, {} played on, {} not dealt for histories apart", tally.compared, tally.pairs, tally.worlds, tally.stand_ins, tally.apart);
}

/// The game a scenario hands over at its question, checked as it plays.
fn handed_over(text: &str) -> Game {
    let script = GameScript::parse(text, "world").expect("parses");
    let result = cantrip_script::run(&script).map_err(|f| f.report).expect("reaches its handover");
    assert!(result.handed_over);
    let mut game = result.game;
    game.verify = true;
    game
}

fn names(game: &Game, zone: Zone) -> Vec<&'static str> {
    let mut names: Vec<&'static str> = game.state.zone(zone).iter().map(|&id| game.cards.get(game.state.obj(id).card).name).collect();
    names.sort();
    names
}

/// A world for Bot2 dealt into a fork of `game`, in Decklist mode.
fn world_for_bot2(game: &Game) -> Game {
    let mut world = game.fork_for_search();
    world.reseed(5);
    determinize(&mut world, &Knowledge::of(game, PlayerId(1)), PlayerId(1), InfoMode::Decklist, prior(), &mut Rng::new(5));
    world
}

/// Bot1's Kaito, Bane of Nightmares's ninjutsu on the stack, activated from Bot1's hand in place of the unblocked Guerrilla
/// Gorilla, Bot2 to answer it; the rest of Bot1's hand and library are Forests.
const NINJUTSU_ON_THE_STACK: &str = "
[Bot1 Deck]
Forest
Mox Emerald
Guerrilla Gorilla
Mox Sapphire
Mox Jet
Kaito, Bane of Nightmares
Forest x2
Forest x10

[Bot2 Deck]
Island x20

[Actions]
Bot1.Hand.Forest.Play
Bot1.Hand.Mox Emerald.Cast
Bot1.Pass
Bot1.Hand.Mox Sapphire.Cast
Bot1.Pass
Bot1.Hand.Mox Jet.Cast
Bot1.Pass
Bot1.Battlefield.Forest.Activate
Bot1.Battlefield.Mox Emerald.Activate
Bot1.Hand.Guerrilla Gorilla.Cast
Bot1.Pass
[T3 Bot1.Main1]
Bot1.Attack(Guerrilla Gorilla)
[T3 Bot1.Blockers]
Bot1.Battlefield.Mox Emerald.Activate
Bot1.Battlefield.Mox Sapphire.Activate
Bot1.Battlefield.Mox Jet.Activate
Bot1.Hand.Kaito, Bane of Nightmares.Activate(Ability: Ninjutsu)
Bot1.Choose(Guerrilla Gorilla)
Bot1.Pass
[Handover Bot2.Pass]
";

/// A card an ability on the stack was activated from in a hand is revealed (CR 602.2a): Bot2 sees Kaito in Bot1's hand
/// while the ninjutsu waits, knows it from then on, and no world dealt for Bot2 makes it another card. One did (the AI
/// host's search panicked resolving the ninjutsu as an ability of the Forest dealt in Kaito's place, CardTable::ability
/// out of range): the deal took the card for unseen and gave the ability on the stack to the card dealt.
#[test]
fn a_card_an_ability_was_activated_from_in_a_hand_is_seen_and_kept_in_every_world() {
    let game = handed_over(NINJUTSU_ON_THE_STACK);
    let (bot1, bot2) = (PlayerId(0), PlayerId(1));
    let item = *game.state.stack.last().expect("the ninjutsu on the stack");
    let kaito = item.object;
    let name = |g: &Game, id: ObjectId| g.cards.get(g.state.obj(id).card).name;
    assert_eq!(game.state.obj(kaito).zone, Zone::hand(bot1));
    assert_eq!(name(&game, kaito), "Kaito, Bane of Nightmares");
    assert!(observable(&game, bot2, &std::collections::HashSet::new()).contains(&kaito), "Bot2 sees the card the ninjutsu revealed");
    let knowledge = Knowledge::of(&game, bot2);
    assert_eq!(knowledge.knows(kaito), Some(false), "Bot2 knows the card in Bot1's hand");
    for seed in 0..32 {
        let mut world = game.fork_for_search();
        world.reseed(seed);
        determinize(&mut world, &knowledge, bot2, InfoMode::Decklist, prior(), &mut Rng::new(seed));
        assert_eq!(name(&world, kaito), "Kaito, Bane of Nightmares", "world {seed} keeps the revealed card");
        assert_eq!(world.state.stack.last().map(|i| i.ability), Some(item.ability), "world {seed} keeps the ability on the stack");
        let request = world.pending().cloned().expect("Bot2's priority");
        world.answer(request.id, request.player, Answer::Pass).expect("legal");
        assert!(
            world.state.battlefield().iter().any(|&id| name(&world, id) == "Kaito, Bane of Nightmares"),
            "world {seed}: the ninjutsu resolves, Kaito onto the battlefield"
        );
    }
}

/// Faithless Looting resolving, its two cards drawn, Bot1 to choose the two it discards: the draws are in the journal's
/// tail the fork keeps, and choosing re-runs the resolution from before them.
const LOOTING_CHOOSES: &str = "
[Bot1 Deck]
Mountain
Faithless Looting
Lightning Bolt
Counterspell
Shock
Duress
Thoughtseize
Giant Growth
Dark Ritual
Opt
Brainstorm
Ponder
Black Lotus
Mox Ruby
Island x6

[Bot2 Deck]
Island x20

[Actions]
Bot1.Hand.Mountain.Play
Bot1.Battlefield.Mountain.Activate
Bot1.Hand.Faithless Looting.Cast
[Handover Bot1.Choose]
";

/// A world dealt while a resolution waits on a question keeps its cards when the answer re-runs the resolution from
/// its start (the cards drawn in it are drawn again, as the world's), and when the fork goes back to its root.
#[test]
fn a_world_dealt_mid_resolution_keeps_its_cards_through_the_rerun_and_back() {
    let game = handed_over(LOOTING_CHOOSES);
    let (state, log) = (game.state.clone(), game.log.clone());
    let bot1 = PlayerId(0);
    let mut world = world_for_bot2(&game);
    let hand = names(&world, Zone::hand(bot1));
    assert_eq!(hand.len(), 7);
    // The two drawn in the resolution, rows the fork's journal tail made.
    let drawn: Vec<ObjectId> = game
        .log
        .iter()
        .filter_map(|r| match r.event {
            Event::ZoneChange { new, from, to, .. } if from == Zone::library(bot1) && to == Zone::hand(bot1) => Some(new),
            _ => None,
        })
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .take(2)
        .collect();
    let card = |g: &Game, id: ObjectId| g.state.obj(id).card;
    assert!(drawn.iter().any(|&id| card(&world, id) != card(&game, id)), "the world deals the cards drawn in the resolution anew");
    // Each left a row as it moved from the library (CR 400.7), which showed its card: in the world, the card dealt.
    for &id in &drawn {
        let from = (0..world.state.objects.len()).map(|i| ObjectId(i as u32)).find(|&o| world.state.obj(o).became == Some(id)).expect("drawn");
        let row = |g: &Game| g.state.last_known(from).expect("a row").name;
        assert_eq!(row(&game), game.cards.get(card(&game, id)).name);
        assert_eq!(row(&world), world.cards.get(card(&world, id)).name, "the row a drawn card left shows the card dealt");
    }
    assert_ne!(hand, names(&game, Zone::hand(bot1)), "the world deals Bot1's hand anew");
    let dealt = (world.state.clone(), world.log.clone());
    let root = world.checkpoint();

    let request = world.pending().cloned().expect("the discard");
    let RequestKind::ChooseCards { cards, rule, .. } = &request.kind else { panic!("a card choice: {request:?}") };
    let (eligible, count) = (rule.among.as_ref().unwrap_or(cards), &rule.count.most().expect("a number"));
    let chosen: Vec<ObjectId> = eligible[..*count as usize].to_vec();
    let discarded: Vec<&str> = chosen.iter().map(|&id| world.cards.get(card(&world, id)).name).collect();
    world.answer(request.id, request.player, Answer::Cards(chosen)).expect("legal");
    let mut kept = hand.clone();
    for d in &discarded {
        kept.remove(kept.iter().position(|n| n == d).expect("discarded from the hand"));
    }
    assert_eq!(names(&world, Zone::hand(bot1)), kept, "the hand after the re-run is the world's, less the two discarded");
    let mut graveyard = discarded.clone();
    graveyard.push("Faithless Looting");
    graveyard.sort();
    assert_eq!(names(&world, Zone::graveyard(bot1)), graveyard);

    world.goto(root);
    assert!(world.state == dealt.0 && world.log == dealt.1, "back at the fork's root, the world as dealt");
    assert!(game.state == state && game.log == log, "the real game is untouched");
}

/// Duress cast, its mana not yet paid: cancelling it reverts the cast to before it began.
const DURESS_PAYS: &str = "
[Bot1 Deck]
Swamp
Duress
Lightning Bolt
Counterspell
Shock
Thoughtseize
Giant Growth
Dark Ritual
Opt
Brainstorm
Ponder
Black Lotus
Mox Ruby
Swamp x7

[Bot2 Deck]
Island x20

[Actions]
Bot1.Hand.Swamp.Play
Bot1.Hand.Duress.Cast(Bot2)
[Handover Bot1.PayCost]
";

/// A world dealt while a spell is being cast keeps its cards when the cast is cancelled (CR 733.1): the hand Duress
/// goes back to is the world's.
/// A library a resolution in progress drew from is dealt anew all the same: the seat does not know its own library's
/// order, the draws in the fork's journal tail notwithstanding. The tail's records of what it took out are renamed with
/// the places, so the re-run the answer makes, and the way back to the fork's root, lay out the world's library.
#[test]
fn a_library_drawn_from_mid_resolution_is_reordered_and_rolls_back_whole() {
    let game = handed_over(LOOTING_CHOOSES);
    let bot1 = PlayerId(0);
    let run = game.state.agenda.last().expect("the resolution waits").log_mark as usize;
    assert!(game.log[run..].iter().any(|r| matches!(r.event, Event::Drew { player, .. } if player == bot1)), "the resolution waiting drew from the library");
    let mut world = game.fork_for_search();
    world.reseed(5);
    determinize(&mut world, &Knowledge::of(&game, bot1), bot1, InfoMode::Decklist, prior(), &mut Rng::new(5));
    // The objects themselves are moved, not only dealt other cards: which object lies where is what the real game's order
    // would otherwise tell.
    let order = |g: &Game| g.state.zone(Zone::library(bot1)).to_vec();
    assert_ne!(order(&world), order(&game), "Bot1's library's objects are in another order in the world");
    let dealt = (world.state.clone(), world.log.clone());
    let root = world.checkpoint();
    let request = world.pending().cloned().expect("the discard");
    let RequestKind::ChooseCards { cards, rule, .. } = &request.kind else { panic!("a card choice: {request:?}") };
    let (eligible, count) = (rule.among.as_ref().unwrap_or(cards), &rule.count.most().expect("a number"));
    let chosen: Vec<ObjectId> = eligible[..*count as usize].to_vec();
    world.answer(request.id, request.player, Answer::Cards(chosen)).expect("legal");
    world.goto(root);
    assert!(world.state == dealt.0 && world.log == dealt.1, "back at the fork's root, the world as dealt");
}

/// Ancestral Recall under Chains of Mephistopheles, Golgari Grave-Troll back in Bot1's hand from the first dredge: the
/// second draw is replaced by a discard, and Bot1 is offered the dredge on the draw Chains gives back. Its run milled six
/// cards from Bot1's library before it asked, and put them back when it stopped to ask: the journal's tail names them.
const CHAINS_DREDGES: &str = "
[Bot1 Deck]
Island
Black Lotus
Mox Sapphire
Chains of Mephistopheles
Entomb
Ancestral Recall
Mox Jet
Golgari Grave-Troll
Forest x12
Swamp x13

[Bot2 Deck]
Swamp x20

[Actions]
Bot1.Hand.Island.Play
Bot1.Hand.Black Lotus.Cast
Bot1.Pass
Bot1.Hand.Mox Sapphire.Cast
Bot1.Pass
Bot1.Hand.Mox Jet.Cast
Bot1.Pass
Bot1.Battlefield.Black Lotus.Activate(Color: Black)
Bot1.Hand.Chains of Mephistopheles.Cast
Bot1.Pass
Bot1.Hand.Entomb.Cast
Bot1.Pass
Bot1.Search(Golgari Grave-Troll)
Bot1.Battlefield.Mox Sapphire.Activate
Bot1.Hand.Ancestral Recall.Cast(Bot1)
Bot1.Replace(Golgari Grave-Troll)
Bot1.Accept
Bot1.Choose(Golgari Grave-Troll)
[Handover Bot1.Accept]
";

/// A library whose objects a resolution waiting on its question took out and put back is dealt anew with them in it (the
/// long test found the real top six kept here, which told the seat the real shuffle): the answer's re-run mills the
/// world's top six, and the way back to the fork's root gives the world as dealt. That a rollback through the tail lays
/// out the new order is `GameState`'s own test (`a_reordered_library_rolls_back_in_its_new_order`).
#[test]
fn a_library_a_waiting_resolution_took_from_and_put_back_is_dealt_anew_with_the_rest() {
    let game = handed_over(CHAINS_DREDGES);
    let bot1 = PlayerId(0);
    let mut world = game.fork_for_search();
    world.reseed(5);
    determinize(&mut world, &Knowledge::of(&game, bot1), bot1, InfoMode::Hidden, prior(), &mut Rng::new(5));
    let library = |g: &Game| g.state.zone(Zone::library(bot1)).to_vec();
    assert_ne!(library(&world), library(&game), "Bot1's library's objects are in another order in the world");
    let top: Vec<ObjectId> = library(&world)[..6].to_vec();
    let dealt = (world.state.clone(), world.log.clone());
    let root = world.checkpoint();
    let request = world.pending().cloned().expect("the dredge offered");
    world.answer(request.id, request.player, Answer::Yes).expect("legal");
    let milled: Vec<ObjectId> = world
        .log
        .iter()
        .rev()
        .filter_map(|r| match r.event {
            Event::ZoneChange { old, from, to, .. } if from == Zone::library(bot1) && to == Zone::graveyard(bot1) => Some(old),
            _ => None,
        })
        .take(6)
        .collect();
    let mut want = top.clone();
    want.sort();
    let mut got = milled.clone();
    got.sort();
    assert_eq!(got, want, "the dredge mills the world's top six");
    world.goto(root);
    assert!(world.state == dealt.0 && world.log == dealt.1, "back at the fork's root, the world as dealt");
}

#[test]
fn a_world_dealt_mid_cast_keeps_its_cards_when_the_cast_is_cancelled() {
    let game = handed_over(DURESS_PAYS);
    let bot1 = PlayerId(0);
    let mut world = world_for_bot2(&game);
    let hand = names(&world, Zone::hand(bot1));
    assert_ne!(hand, names(&game, Zone::hand(bot1)), "the world deals Bot1's hand anew");
    let dealt = (world.state.clone(), world.log.clone());
    let root = world.checkpoint();

    let request = world.pending().cloned().expect("the payment");
    world.answer(request.id, request.player, Answer::Cancel).expect("legal");
    assert!(world.state.stack.is_empty());
    let mut back = hand.clone();
    back.push("Duress");
    back.sort();
    assert_eq!(names(&world, Zone::hand(bot1)), back, "the hand after the cancel is the world's, and Duress");

    world.goto(root);
    assert!(world.state == dealt.0 && world.log == dealt.1, "back at the fork's root, the world as dealt");
}

/// A world dealt for Bot1 while Bot1 chooses who goes first, before any companion is revealed (CR 103.2b): two games that
/// differ only in the companion Bot2 chose at deck building - a Lurrus of the Dream-Den its deck of Islands fulfils, or the
/// same Lurrus a plain sideboard card - give the same world, and once the starting player is chosen neither world reveals
/// anything of Bot2's: what Bot2 chose is Bot2's alone until it is revealed.
#[test]
fn a_companion_not_yet_revealed_is_not_in_another_seats_world() {
    let lurrus = ids(&["Lurrus of the Dream-Den".to_string()])[0];
    let swamps = ids(&vec!["Swamp".to_string(); 40]);
    let islands = ids(&vec!["Island".to_string(); 40]);
    let game = |chosen: bool| {
        let setup = Setup {
            decks: vec![swamps.clone(), islands.clone()],
            sideboards: vec![vec![], if chosen { vec![] } else { vec![lurrus] }],
            companions: vec![None, chosen.then_some(lurrus)],
            shuffle: false,
            opening_chooser: Some(PlayerId(0)),
            ..Setup::default()
        };
        let mut game = Game::new(cards(), &setup);
        game.run();
        assert!(matches!(game.pending().map(|r| &r.kind), Some(RequestKind::ChooseStartingPlayer { .. })));
        game
    };
    let (a, b) = (game(true), game(false));
    assert_eq!(a.state.player(PlayerId(1)).companion_to_reveal.is_some(), true, "Bot2's companion is to be revealed in game A");
    let world = |g: &Game| {
        let mut world = g.fork_for_search();
        world.reseed(5);
        determinize(&mut world, &Knowledge::of(g, PlayerId(0)), PlayerId(0), InfoMode::Hidden, prior(), &mut Rng::new(5));
        world
    };
    let (mut wa, mut wb) = (world(&a), world(&b));
    assert_eq!(wa.state.players, wb.state.players, "the worlds' players differ before the reveal");
    assert_eq!(canon(&wa), canon(&wb));
    for w in [&mut wa, &mut wb] {
        let request = w.pending().expect("who goes first").id;
        w.answer(request, PlayerId(0), Answer::Player(PlayerId(0))).expect("Bot1 goes first");
    }
    assert_eq!(canon_of(&wa, false), canon_of(&wb, false), "a world revealed what Bot2 chose");
    assert_eq!(wa.state.player(PlayerId(1)).companion, None);
}

/// Cryptic Coat cloaks Bot1's top card, a Snapcaster Mage Bot2 never sees (CR 701.58a), and the face-down creature attacks
/// on turn 3: its declaration keeps it as it attacked, a nameless 2/2 equipped to 3/2, in a last-known row
/// (`Event::Attacks::known`). Bot1 then has priority in its second main phase.
const CLOAKED_ATTACKER: &str = "
[Bot1 Deck]
Black Lotus
Cryptic Coat
Island x5
Snapcaster Mage
Island x10

[Bot2 Deck]
Island x20

[Actions]
Bot1.Hand.Black Lotus.Cast
Bot1.Battlefield.Black Lotus.Activate(Color: Blue)
Bot1.Hand.Cryptic Coat.Cast
Bot1.Pass
Bot2.Pass
Bot1.Pass
Bot2.Pass
[T3 Bot1.Main1]
Bot1.Attack(#1 > Bot2)
[T3 Bot1.Main2]
[Handover Bot1.Pass]

[Expect]
Turn = 3
Step = Main2
Bot2.Life = 17
";

/// A world for Bot2 deals the cloaked attacker anew (`GameState::rebind_hidden`): the row its attack kept was of a
/// face-down creature, which shows nothing of the card it is dealt from, so it may be (the rebind's checks hold), and
/// the attacker as declared is the same in the world as in the real game.
#[test]
fn a_world_deals_anew_a_face_down_creature_whose_attack_kept_it_as_declared() {
    let game = handed_over(CLOAKED_ATTACKER);
    let bot1 = PlayerId(0);
    let declared = |g: &Game| -> Vec<(Option<i16>, Option<i16>, &'static str)> {
        g.history()
            .attacked_with(bot1)
            .map(|a| {
                let c = a.chars(g);
                (c.power, c.toughness, c.name)
            })
            .collect()
    };
    let real = declared(&game);
    assert_eq!(real.len(), 1, "the cloaked creature attacked");
    assert_eq!((real[0].0, real[0].1), (Some(3), Some(2)), "a face-down 2/2 equipped with Cryptic Coat as it attacked");
    let world = world_for_bot2(&game);
    let attacker = game.history().attacked_with(bot1).next().expect("the attack").attacker;
    assert_ne!(world.state.obj(attacker).card, game.state.obj(attacker).card, "the world deals the cloaked card anew");
    // The library card it was cloaked from left a row as it moved (CR 400.7), already face down (CR 701.58a): nameless in
    // the game and in the world alike.
    let from = (0..world.state.objects.len()).map(|i| ObjectId(i as u32)).find(|&id| world.state.obj(id).became == Some(attacker)).expect("cloaked from the library");
    assert_eq!(game.state.last_known(from).expect("a row").name, "");
    assert_eq!(world.state.last_known(from).expect("a row").name, "");
    assert_eq!(declared(&world), real, "the world's attacker as declared is the real one's: nothing of its face");
}
