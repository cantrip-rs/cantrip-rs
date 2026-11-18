//! `cantrip-fingerprint`: what each card does to a game, for the network's card input (the card-representation
//! experiment): a card is played, in every way it can be, in a few fixed positions, and what changed is measured.
//!
//! `cantrip-fingerprint --names FILE --out FILE [--threads N] [--budget N] [--probes a,b]`: one JSON line per name
//! (`name`, and per probe `roots`, `leaves` and the `max`, `min` and `mean` of each leaf feature), after a first line
//! naming the features (`{"features": [...], "probes": [...]}`). `FILE` holds one card name a line.
//!
//! A probe position is a scripted game (`cantrip-script`) with the card in Bot1's opening hand: Gleemox for mana of any
//! color (nine at the probe), a Grizzly Bears of Bot1's own and an Island in hand; Bot2's board built from vanilla creatures. At the
//! position, each action the card offers (a cast by each way, an ability activated from the hand, a land play) is a
//! root. Bot1's choices until it has resolved (targets, modes, X, what resolving asks) are branched, the leaf budget
//! split across the options; Bot2 passes, declines and takes the first legal answer. Once it has resolved, both seats
//! answer by default (pass, no attack, no block) to the start of the next turn, and the leaf is measured against the
//! position.

#[cfg(windows)]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

use std::collections::{HashMap, HashSet};
use std::io::{BufRead, Write};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::mpsc::channel;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use cantrip_ai::{Answering, Knowledge, observe_indexed};
use cantrip_ai_host::STACK_SIZE;
use cantrip_engine::*;
use cantrip_obs::InfoMode;
use cantrip_obs::factor::{FactoredAnswer, Machine};
use cantrip_script::{Advance, GameScript, Scripted};
use serde_json::json;

const ME: PlayerId = PlayerId(0);
const OPP: PlayerId = PlayerId(1);
/// The requests one leaf may take before it is cut (`capped`).
const MAX_STEPS: usize = 800;

// ---------------------------------------------------------------------------------------------------------------
// The probe positions

#[derive(Clone, Copy, PartialEq, Eq)]
enum Stop {
    /// Bot1's first priority in its main phase of this turn, the stack empty.
    Main(u32),
    /// Bot1's first priority on this turn with something on the stack.
    Stack(u32),
    /// Bot1's first priority in the declare attackers step of this turn.
    Attacked(u32),
}

struct Probe {
    name: &'static str,
    /// Bot2's sixth-turn draw.
    draw6: &'static str,
    /// Whether Bot2 builds its board on turns 2 and 4.
    board: bool,
    /// Bot2's lines on turn 6.
    turn6: &'static str,
    life2: Option<u32>,
    stop: Stop,
}

/// `n` Gleemox of `bot`'s tapped for `color`, the mana left floating for the next cast.
fn tap(bot: &str, color: &str, n: usize) -> String {
    format!("{bot}.Battlefield.Gleemox.Activate(Color: {color})\n").repeat(n)
}

fn cast(bot: &str, name: &str) -> String {
    format!("{bot}.Hand.{name}.Cast\n")
}

const CASTS6: &str = "Caelorna and the Wind Drake";

const PROBES: &[Probe] = &[
    Probe {
        name: "empty",
        draw6: "Island",
        board: false,
        turn6: "",
        life2: None,
        stop: Stop::Main(7),
    },
    Probe {
        name: "board",
        draw6: "Wind Drake",
        board: true,
        turn6: CASTS6,
        life2: None,
        stop: Stop::Main(7),
    },
    Probe {
        name: "low",
        draw6: "Wind Drake",
        board: true,
        turn6: CASTS6,
        life2: Some(3),
        stop: Stop::Main(7),
    },
    Probe {
        name: "stack-creature",
        draw6: "Wind Drake",
        board: true,
        turn6: "Wind Drake",
        life2: None,
        stop: Stop::Stack(6),
    },
    Probe {
        name: "stack-spell",
        draw6: "Shock",
        board: true,
        turn6: "Shock",
        life2: None,
        stop: Stop::Stack(6),
    },
    // Spells on the stack a narrower counter needs: mana value 2, an artifact, mana value 4.
    Probe {
        name: "stack-two",
        draw6: "Grizzly Bears",
        board: true,
        turn6: "Grizzly Bears",
        life2: None,
        stop: Stop::Stack(6),
    },
    Probe {
        name: "stack-artifact",
        draw6: "Darksteel Myr",
        board: true,
        turn6: "Darksteel Myr",
        life2: None,
        stop: Stop::Stack(6),
    },
    Probe {
        name: "stack-four",
        draw6: "Hill Giant",
        board: true,
        turn6: "Hill Giant",
        life2: None,
        stop: Stop::Stack(6),
    },
    Probe {
        name: "combat",
        draw6: "Island",
        board: true,
        turn6: "attack",
        life2: None,
        stop: Stop::Attacked(6),
    },
];

/// The probe's script with `card` in Bot1's opening hand, its lines in the order of the turns (a script has one
/// cursor). Bot1 casts its Gleemox, Ancestral Recall and a Grizzly Bears; Bot2 its Gleemox, then (but in the empty probe) Grizzly Bears
/// and Darksteel Myr on turn 2, Hill Giant on turn 4, and on turn 6 the probe's own lines.
fn script(probe: &Probe, card: &str) -> String {
    let mut s = String::new();
    // Ancestral Recall on the first turn draws three more Gleemox: nine at the probe, for the costliest cards.
    s.push_str(&format!(
        "[Bot1 Deck]\n{card}\n4 Gleemox\nAncestral Recall\nGrizzly Bears\n3 Gleemox\nGleemox\nGleemox\nIsland\n20 Island\n"
    ));
    s.push_str(&format!(
        "[Bot2 Deck]\n5 Gleemox\nGrizzly Bears\nDarksteel Myr\nHill Giant\nCaelorna, Coral Tyrant\n{}\n20 Island\n",
        probe.draw6
    ));
    if let Some(life) = probe.life2 {
        s.push_str(&format!("[Life]\nBot2: {life}\n"));
    }
    s.push_str("[Actions]\n");
    s.push_str(&cast("Bot1", "Gleemox").repeat(4));
    s.push_str(&tap("Bot1", "Blue", 1));
    s.push_str("Bot1.Hand.Ancestral Recall.Cast(Bot1)\n");
    s.push_str(&cast("Bot1", "Gleemox").repeat(3));
    s.push_str(&tap("Bot1", "Green", 2));
    s.push_str(&cast("Bot1", "Grizzly Bears"));
    s.push_str("[T2 Bot2.Main1]\n");
    s.push_str(&cast("Bot2", "Gleemox").repeat(5));
    if probe.board {
        s.push_str(&tap("Bot2", "Green", 2));
        s.push_str(&cast("Bot2", "Grizzly Bears"));
        s.push_str(&tap("Bot2", "Green", 3));
        s.push_str(&cast("Bot2", "Darksteel Myr"));
    }
    s.push_str("[T3 Bot1.Main1]\n");
    s.push_str(&cast("Bot1", "Gleemox"));
    if probe.board {
        s.push_str("[T4 Bot2.Main1]\n");
        s.push_str(&tap("Bot2", "Red", 4));
        s.push_str(&cast("Bot2", "Hill Giant"));
    }
    s.push_str("[T5 Bot1.Main1]\n");
    s.push_str(&cast("Bot1", "Gleemox"));
    match probe.turn6 {
        "" => {}
        CASTS6 => {
            s.push_str("[T6 Bot2.Main1]\n");
            s.push_str(&tap("Bot2", "Blue", 2));
            s.push_str(&cast("Bot2", "Caelorna, Coral Tyrant"));
            s.push_str(&tap("Bot2", "Blue", 3));
            s.push_str(&cast("Bot2", "Wind Drake"));
        }
        creature @ ("Wind Drake" | "Grizzly Bears" | "Darksteel Myr" | "Hill Giant") => {
            let (color, n) = match creature {
                "Wind Drake" => ("Blue", 3),
                "Grizzly Bears" => ("Green", 2),
                "Darksteel Myr" => ("Green", 3),
                _ => ("Red", 4),
            };
            s.push_str("[T6 Bot2.Main1]\n");
            s.push_str(&tap("Bot2", color, n));
            s.push_str(&cast("Bot2", creature));
        }
        "Shock" => {
            s.push_str("[T6 Bot2.Main1]\n");
            s.push_str(&tap("Bot2", "Red", 1));
            s.push_str("Bot2.Hand.Shock.Cast(Bot1.Battlefield.Grizzly Bears)\n");
        }
        "attack" => {
            s.push_str("[T6 Bot2.Main1]\nBot2.Attack(Grizzly Bears > Bot1; Hill Giant > Bot1)\n")
        }
        other => unreachable!("{other}"),
    }
    s
}

fn stopped(game: &Game, request: &Request, stop: Stop) -> bool {
    if request.player != ME || !matches!(request.kind, RequestKind::Priority { .. }) {
        return false;
    }
    let turn = &game.state.turn;
    let stack = !game.state.zone(Zone::STACK).is_empty();
    match stop {
        Stop::Main(n) => turn.number == n && turn.step() == Step::PrecombatMain && !stack,
        Stop::Stack(n) => turn.number == n && stack,
        Stop::Attacked(n) => turn.number == n && turn.step() == Step::DeclareAttackers,
    }
}

/// The game at the probe's position, Bot1 to act: the script played (Bot1's questions answered as it means them),
/// then both seats by default until the position comes.
fn position(probe: &Probe, card: &str) -> Result<Game, String> {
    let text = script(probe, card);
    let parsed = GameScript::parse(&text, probe.name).map_err(|e| format!("parse: {e:?}"))?;
    let mut run = Scripted::deal(parsed).map_err(|e| format!("deal: {}", e.message))?;
    loop {
        match run
            .advance(&[ME])
            .map_err(|e| format!("script: {}", e.message))?
        {
            Advance::Ask(expected) => {
                if expected.line.is_none() && stopped(run.game(), &expected.request, probe.stop) {
                    return Ok(std::mem::replace(run.game(), placeholder()));
                }
                run.answered(expected.intent)
                    .map_err(|e| format!("answer: {}", e.message))?;
            }
            Advance::Check(_) => {}
            Advance::Done => break,
        }
    }
    let mut game = std::mem::replace(run.game(), placeholder());
    let mut k = [Knowledge::new(ME), Knowledge::new(OPP)];
    for _ in 0..MAX_STEPS {
        let Some(request) = game.pending().cloned() else {
            return Err("no request".into());
        };
        if stopped(&game, &request, probe.stop) {
            return Ok(game);
        }
        let answer = default_answer(&game, &request, &mut k)
            .ok_or("no default answer before the position")?;
        game.answer(request.id, request.player, answer)
            .map_err(|e| format!("refused: {e:?}"))?;
    }
    Err("the position never came".into())
}

fn placeholder() -> Game {
    cantrip_engine::deal(
        &[vec!["Island".into()], vec!["Island".into()]],
        0,
        false,
        Some(ME),
    )
    .expect("deals")
}

// ---------------------------------------------------------------------------------------------------------------
// Answers

fn answering(game: &Game, request: &Request, k: &mut [Knowledge; 2]) -> Option<Answering> {
    let seat = request.player;
    k[seat.index()].update(game);
    let (obs, index) = observe_indexed(game, &k[seat.index()], seat, InfoMode::Full)?;
    Some(Answering::new(game, request, &obs, &index))
}

/// The answer a seat gives by default: pass, no attack, no block, decline, and otherwise the first legal answer the
/// factoring makes that the engine takes.
fn default_answer(game: &Game, request: &Request, k: &mut [Knowledge; 2]) -> Option<Answer> {
    match &request.kind {
        RequestKind::Priority { .. } => return Some(Answer::Pass),
        RequestKind::DeclareAttackers { .. } => return Some(Answer::Attack(Vec::new())),
        RequestKind::DeclareBlockers { .. } => return Some(Answer::Block(Vec::new())),
        RequestKind::May { .. } => return Some(Answer::No),
        RequestKind::PayCosts {
            announcing: false, ..
        } => return cantrip_ai::pay::pay(game, request, None),
        RequestKind::UnlessPays { options, .. } => {
            if let Some(o) = options.iter().find(|o| o.kind == ActionKind::DeclineCost) {
                return Some(Answer::Act(Action::of(o)));
            }
        }
        _ => {}
    }
    first_legal(game, request, k)
}

/// The first answer of the factoring the engine takes, backing off the deepest choice that had another option when it
/// refuses one (as the network player falls back).
fn first_legal(game: &Game, request: &Request, k: &mut [Knowledge; 2]) -> Option<Answer> {
    let answering = answering(game, request, k)?;
    let mut banned: HashSet<(Vec<cantrip_obs::factor::Token>, cantrip_obs::factor::Token)> =
        HashSet::new();
    for _ in 0..16 {
        let mut m = Machine::new(&answering.ask, &[]);
        let mut walk = Vec::new();
        while let Some(step) = m.step() {
            let prefix = m.chosen().to_vec();
            let legal: Vec<usize> = step.legal().collect();
            let open: Vec<usize> = legal
                .iter()
                .copied()
                .filter(|&i| !banned.contains(&(prefix.clone(), step.tokens[i])))
                .collect();
            let open = if open.is_empty() { legal } else { open };
            let i = *open.first()?;
            walk.push((prefix, step.tokens[i], open.len() > 1));
            m.choose(i);
        }
        let factored = m.answer()?.clone();
        match answering.engine(game, request, &factored) {
            Ok(a) => {
                if !cancels(&a) {
                    return Some(a);
                }
                let (prefix, token, _) = walk.into_iter().rev().find(|w| w.2)?;
                banned.insert((prefix, token));
            }
            Err(_) => {
                let (prefix, token, _) = walk.into_iter().rev().find(|w| w.2)?;
                banned.insert((prefix, token));
            }
        }
    }
    None
}

/// Every complete answer of the machine, the budget split across each step's options (`budget` or more leaves: one
/// each, evenly spaced, when there are more options than budget).
fn answers(m: Machine, budget: usize, out: &mut Vec<(FactoredAnswer, usize)>) {
    let mut m = m;
    loop {
        let Some(step) = m.step() else {
            if let Some(f) = m.answer() {
                out.push((f.clone(), budget));
            }
            return;
        };
        let legal: Vec<usize> = step.legal().collect();
        match legal.len() {
            0 => return,
            1 => m.choose(legal[0]),
            n => {
                let picks: Vec<(usize, usize)> = if budget >= n {
                    (0..n)
                        .map(|j| (legal[j], budget / n + usize::from(j < budget % n)))
                        .collect()
                } else {
                    (0..budget.max(1))
                        .map(|j| (legal[j * n / budget.max(1)], 1))
                        .collect()
                };
                for (i, b) in picks {
                    let mut c = m.clone();
                    c.choose(i);
                    answers(c, b, out);
                }
                return;
            }
        }
    }
}

// ---------------------------------------------------------------------------------------------------------------
// What a leaf measures

/// The permanents tracked by name and controller, in feature order.
const TRACKED: &[(PlayerId, &str)] = &[
    (OPP, "Grizzly Bears"),
    (OPP, "Darksteel Myr"),
    (OPP, "Hill Giant"),
    (OPP, "Caelorna, Coral Tyrant"),
    (OPP, "Wind Drake"),
    (ME, "Grizzly Bears"),
    (OPP, "Gleemox"),
    (ME, "Gleemox"),
];

const SLOT: &[&str] = &[
    "present",
    "gone",
    "graveyard",
    "exile",
    "hand",
    "library",
    "returned",
    "control",
    "power",
    "toughness",
    "damage",
    "tapped",
    "counters",
    "keywords_gained",
    "keywords_lost",
];
const PLAYER: &[&str] = &[
    "life",
    "hand_new",
    "hand_gone",
    "library",
    "graveyard",
    "exile",
    "poison",
];
const NEW: &[&str] = &[
    "count",
    "creatures",
    "tokens",
    "lands",
    "artifacts",
    "enchantments",
    "planeswalkers",
    "power",
    "toughness",
    "max_power",
    "max_toughness",
];
const KEYWORDS: usize = 33;

fn feature_names() -> Vec<String> {
    let mut f: Vec<String> = [
        "won",
        "lost",
        "capped",
        "gleemox_tapped",
        "emblems",
        "stack_left",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect();
    for side in ["me", "opp"] {
        f.extend(PLAYER.iter().map(|p| format!("{side}.{p}")));
    }
    for (p, name) in TRACKED {
        let side = if *p == ME { "me" } else { "opp" };
        f.extend(SLOT.iter().map(|s| format!("{side}.{name}.{s}")));
    }
    for side in ["me", "opp"] {
        f.extend(NEW.iter().map(|n| format!("{side}.new.{n}")));
    }
    f.extend((0..KEYWORDS).map(|i| format!("me.new.keyword{i}")));
    let later: Vec<String> = f.iter().map(|n| format!("later.{n}")).collect();
    f.extend(later);
    f
}

struct Perm {
    controller: PlayerId,
    power: i32,
    toughness: i32,
    damage: u32,
    tapped: bool,
    counters: i32,
    keywords: u128,
}

fn perm(game: &Game, id: ObjectId) -> Perm {
    let o = game.state.obj(id);
    let c = game.chars(id);
    Perm {
        controller: o.controller,
        power: c.power.unwrap_or(0),
        toughness: c.toughness.unwrap_or(0),
        damage: o.damage,
        tapped: o.tapped,
        counters: o.counter_array().iter().sum(),
        keywords: c.keywords.0,
    }
}

/// The position a leaf is measured against.
struct Snapshot {
    turn: u32,
    probe_card: ObjectId,
    life: [i32; 2],
    poison: [u8; 2],
    hands: [HashSet<ObjectId>; 2],
    sizes: [[usize; 3]; 2],
    battlefield: HashSet<ObjectId>,
    stack: HashSet<ObjectId>,
    tracked: Vec<Option<(ObjectId, Perm)>>,
    gleemox_untapped: usize,
    emblems: usize,
}

fn sizes(game: &Game, p: PlayerId) -> [usize; 3] {
    [
        game.state.zone(Zone::library(p)).len(),
        game.state.zone(Zone::graveyard(p)).len(),
        game.state.zone(Zone::exile(p)).len(),
    ]
}

fn untapped_gleemox(game: &Game) -> usize {
    game.state
        .battlefield()
        .iter()
        .filter(|&&id| {
            game.state.obj(id).controller == ME
                && !game.state.obj(id).tapped
                && game.name(id) == "Gleemox"
        })
        .count()
}

fn snapshot(game: &Game, probe_card: ObjectId) -> Snapshot {
    let s = &game.state;
    let tracked = TRACKED
        .iter()
        .map(|&(p, name)| {
            s.battlefield()
                .iter()
                .copied()
                .find(|&id| s.obj(id).controller == p && game.name(id) == name)
                .map(|id| (id, perm(game, id)))
        })
        .collect();
    Snapshot {
        turn: s.turn.number,
        probe_card,
        life: [s.player(ME).life, s.player(OPP).life],
        poison: [s.player(ME).poison(), s.player(OPP).poison()],
        hands: [
            s.zone(Zone::hand(ME)).iter().copied().collect(),
            s.zone(Zone::hand(OPP)).iter().copied().collect(),
        ],
        sizes: [sizes(game, ME), sizes(game, OPP)],
        battlefield: s.battlefield().iter().copied().collect(),
        stack: s.zone(Zone::STACK).iter().copied().collect(),
        tracked,
        gleemox_untapped: untapped_gleemox(game),
        emblems: s.zone(Zone::COMMAND).len(),
    }
}

/// Where an object that left has gone: the last object of its chain of `became`.
fn last_of(game: &Game, id: ObjectId) -> ObjectId {
    let mut id = id;
    for _ in 0..64 {
        match game.state.obj(id).became {
            Some(next) if next != id => id = next,
            _ => break,
        }
    }
    id
}

fn features(game: &Game, snap: &Snapshot, capped: bool) -> Vec<f32> {
    let s = &game.state;
    let mut f = Vec::with_capacity(256);
    let (won, lost) = match s.outcome {
        Outcome::Won(p) => (f32::from(p == ME), f32::from(p != ME)),
        _ => (0.0, 0.0),
    };
    f.extend([won, lost, f32::from(capped)]);
    f.push(snap.gleemox_untapped as f32 - untapped_gleemox(game) as f32);
    f.push((s.zone(Zone::COMMAND).len() as f32) - snap.emblems as f32);
    f.push(s.zone(Zone::STACK).len() as f32);
    for (i, p) in [ME, OPP].into_iter().enumerate() {
        let hand: HashSet<ObjectId> = s.zone(Zone::hand(p)).iter().copied().collect();
        let new = hand.difference(&snap.hands[i]).count();
        let gone = snap.hands[i]
            .iter()
            .filter(|&&id| !hand.contains(&id) && id != snap.probe_card)
            .count();
        let now = sizes(game, p);
        f.extend([
            (s.player(p).life - snap.life[i]) as f32,
            new as f32,
            gone as f32,
            now[0] as f32 - snap.sizes[i][0] as f32,
            now[1] as f32 - snap.sizes[i][1] as f32,
            now[2] as f32 - snap.sizes[i][2] as f32,
            f32::from(s.player(p).poison()) - f32::from(snap.poison[i]),
        ]);
    }
    for t in &snap.tracked {
        let Some((id, before)) = t else {
            f.extend(std::iter::repeat_n(0.0, SLOT.len()));
            continue;
        };
        let here = s.battlefield().contains(id);
        let mut slot = vec![0.0f32; SLOT.len()];
        slot[0] = 1.0;
        if here {
            let now = perm(game, *id);
            slot[7] = f32::from(now.controller != before.controller);
            slot[8] = (now.power - before.power) as f32;
            slot[9] = (now.toughness - before.toughness) as f32;
            slot[10] = now.damage as f32 - before.damage as f32;
            slot[11] = f32::from(now.tapped != before.tapped);
            slot[12] = (now.counters - before.counters) as f32;
            slot[13] = (now.keywords & !before.keywords).count_ones() as f32;
            slot[14] = (before.keywords & !now.keywords).count_ones() as f32;
        } else {
            slot[1] = 1.0;
            let to = s.obj(last_of(game, *id)).zone.kind();
            match to {
                ZoneKind::Graveyard => slot[2] = 1.0,
                ZoneKind::Exile => slot[3] = 1.0,
                ZoneKind::Hand => slot[4] = 1.0,
                ZoneKind::Library => slot[5] = 1.0,
                ZoneKind::Battlefield => slot[6] = 1.0,
                _ => {}
            }
        }
        f.extend(slot);
    }
    let mut keywords = 0u128;
    for p in [ME, OPP] {
        let mut n = [0.0f32; 11];
        for &id in s.battlefield() {
            if snap.battlefield.contains(&id) || s.obj(id).controller != p {
                continue;
            }
            let c = game.chars(id);
            let types = c.types;
            n[0] += 1.0;
            n[1] += f32::from(types.has(CardType::Creature));
            n[2] += f32::from(s.obj(id).token);
            n[3] += f32::from(types.has(CardType::Land));
            n[4] += f32::from(types.has(CardType::Artifact));
            n[5] += f32::from(types.has(CardType::Enchantment));
            n[6] += f32::from(types.has(CardType::Planeswalker));
            let (pw, th) = (c.power.unwrap_or(0) as f32, c.toughness.unwrap_or(0) as f32);
            n[7] += pw;
            n[8] += th;
            n[9] = n[9].max(pw);
            n[10] = n[10].max(th);
            if p == ME {
                keywords |= c.keywords.0;
            }
        }
        f.extend(n);
    }
    f.extend((0..KEYWORDS).map(|i| ((keywords >> i) & 1) as f32));
    let _ = &snap.stack;
    f
}

// ---------------------------------------------------------------------------------------------------------------
// The leaves

struct Explorer<'a> {
    snap: &'a Snapshot,
    leaves: Vec<Vec<f32>>,
    /// Whether a leaf plays on through Bot1's next turn (`follow_up`) for its second block of features.
    follow: bool,
}

impl Explorer<'_> {
    /// Records a leaf: the features now, then (where the probe follows up) those after Bot1's next turn.
    fn leaf(&mut self, game: &mut Game, k: &mut [Knowledge; 2], capped: bool) {
        let now = features(game, self.snap, capped);
        let later = if self.follow && !capped && !game.is_over() {
            follow_up(game, k, self.snap);
            features(game, self.snap, false)
        } else if self.follow {
            now.clone()
        } else {
            vec![0.0; now.len()]
        };
        self.leaves.push([now, later].concat());
    }

    /// Plays on from where `game` is, branching on Bot1's choices until its action has resolved, and records a leaf at
    /// the probe turn's end step (or the game's end, or the cap).
    fn explore(
        &mut self,
        game: &mut Game,
        k: &mut [Knowledge; 2],
        budget: usize,
        mut settled: bool,
        mut steps: usize,
    ) {
        loop {
            steps += 1;
            let turn = &game.state.turn;
            let ended = turn.number > self.snap.turn
                || (settled && turn.number == self.snap.turn && turn.step() >= Step::End);
            if game.is_over() || ended || steps > MAX_STEPS {
                self.leaf(game, k, steps > MAX_STEPS);
                return;
            }
            let Some(request) = game.pending().cloned() else {
                self.leaf(game, k, true);
                return;
            };
            let mine = request.player == ME;
            let priority = matches!(request.kind, RequestKind::Priority { .. });
            if mine && priority && !settled {
                let stack: Vec<ObjectId> = game.state.zone(Zone::STACK).to_vec();
                if stack.iter().all(|id| self.snap.stack.contains(id)) {
                    settled = true;
                }
            }
            let paying = matches!(
                &request.kind,
                RequestKind::PayCosts {
                    announcing: false,
                    ..
                }
            );
            if mine && !settled && !priority && !paying {
                let Some(a) = answering(game, &request, k) else {
                    self.leaf(game, k, true);
                    return;
                };
                let mut found = Vec::new();
                answers(Machine::new(&a.ask, &[]), budget, &mut found);
                let node = game.checkpoint();
                let mut any = false;
                for (f, share) in found {
                    game.goto(node);
                    let Ok(answer) = a.engine(game, &request, &f) else {
                        continue;
                    };
                    if cancels(&answer) || game.answer(request.id, request.player, answer).is_err()
                    {
                        continue;
                    }
                    any = true;
                    if debug() {
                        eprintln!("  branch {:?}", kind_label(&request.kind));
                    }
                    let mut k2 = k.clone();
                    self.explore(game, &mut k2, share, settled, steps);
                }
                game.goto(node);
                if !any {
                    self.leaf(game, k, true);
                }
                return;
            }
            let Some(answer) = default_answer(game, &request, k) else {
                if debug() {
                    eprintln!("  no default answer: {:?}", request.kind);
                }
                self.leaf(game, k, true);
                return;
            };
            if debug() {
                eprintln!(
                    "  T{} {:?} {:?} {} -> {:?}",
                    game.state.turn.number,
                    game.state.turn.step(),
                    request.player,
                    kind_label(&request.kind),
                    answer
                );
            }
            if game.answer(request.id, request.player, answer).is_err() {
                self.leaf(game, k, true);
                return;
            }
        }
    }
}

/// What a permanent does after it has resolved: the game played on through Bot1's next turn, where Bot1 plays a land
/// (landfall), attacks Bot2 with the permanents the leaf made, and says yes to its own "you may"s; Bot2 answers by default
/// (no attack, no block).
fn follow_up(game: &mut Game, k: &mut [Knowledge; 2], snap: &Snapshot) {
    let turn = snap.turn + 2;
    let mut played = false;
    for _ in 0..MAX_STEPS {
        let t = &game.state.turn;
        if game.is_over() || t.number > turn || (t.number == turn && t.step() >= Step::End) {
            return;
        }
        let Some(request) = game.pending().cloned() else {
            return;
        };
        let now = game.state.turn.number == turn;
        let answer = match &request.kind {
            _ if request.player != ME => default_answer(game, &request, k),
            RequestKind::Priority { options }
                if now
                    && !played
                    && game.state.turn.step() == Step::PrecombatMain
                    && game.state.zone(Zone::STACK).is_empty() =>
            {
                played = true;
                Some(
                    options
                        .iter()
                        .find(|o| o.kind == ActionKind::PlayLand)
                        .map_or(Answer::Pass, |o| Answer::Act(Action::of(o))),
                )
            }
            RequestKind::DeclareAttackers { eligible, .. } if now => {
                let attack: Vec<(ObjectId, Target)> = eligible
                    .iter()
                    .filter(|id| !snap.battlefield.contains(id))
                    .map(|&id| (id, Target::Player(OPP)))
                    .collect();
                if !attack.is_empty()
                    && game
                        .answer(request.id, request.player, Answer::Attack(attack))
                        .is_ok()
                {
                    continue;
                }
                Some(Answer::Attack(Vec::new()))
            }
            RequestKind::May { .. } => Some(Answer::Yes),
            _ => default_answer(game, &request, k),
        };
        let Some(answer) = answer else { return };
        if game.answer(request.id, request.player, answer).is_err() {
            return;
        }
    }
}

struct Measured {
    roots: usize,
    leaves: Vec<Vec<f32>>,
}

/// Every root action of the card in Bot1's hand at the position, and the leaves under each.
fn measure(game: &mut Game, card: CardId, budget: usize, follow: bool) -> Result<Measured, String> {
    let request = game
        .pending()
        .cloned()
        .ok_or("no request at the position")?;
    let RequestKind::Priority { options } = &request.kind else {
        return Err("not a priority".into());
    };
    let hand = game.state.zone(Zone::hand(ME)).to_vec();
    let fronts = &cantrip_engine::cards().fronts;
    let front = |c: CardId| fronts[c.0 as usize].unwrap_or(c);
    let probe = hand
        .iter()
        .copied()
        .find(|&id| front(game.state.obj(id).card) == front(card))
        .ok_or("the card is not in the hand")?;
    let roots: Vec<ActionOption> = options
        .iter()
        .filter(|o| o.source == Some(probe))
        .filter(|o| {
            matches!(
                o.kind,
                ActionKind::PlayLand | ActionKind::CastSpell | ActionKind::ActivateAbility
            )
        })
        .cloned()
        .collect();
    let snap = snapshot(game, probe);
    let mut ex = Explorer {
        snap: &snap,
        leaves: Vec::new(),
        follow,
    };
    let node = game.checkpoint();
    let mut k = [Knowledge::new(ME), Knowledge::new(OPP)];
    for root in &roots {
        game.goto(node);
        let filtered = Request {
            kind: RequestKind::Priority {
                options: vec![root.clone()],
            },
            ..request.clone()
        };
        let Some(a) = answering(game, &filtered, &mut k) else {
            continue;
        };
        let mut found = Vec::new();
        answers(Machine::new(&a.ask, &[]), budget, &mut found);
        for (f, share) in found {
            game.goto(node);
            let Ok(answer) = a.engine(game, &filtered, &f) else {
                if debug() {
                    eprintln!("root refused: {f:?}");
                }
                continue;
            };
            if matches!(answer, Answer::Pass)
                || cancels(&answer)
                || game.answer(request.id, request.player, answer).is_err()
            {
                continue;
            }
            let mut k2 = k.clone();
            if debug() {
                eprintln!(
                    "root answered; now {:?}",
                    game.pending().map(|r| kind_label(&r.kind))
                );
            }
            ex.explore(game, &mut k2, share, false, 0);
        }
    }
    game.goto(node);
    Ok(Measured {
        roots: roots.len(),
        leaves: ex.leaves,
    })
}

fn summary(m: &Measured, width: usize) -> serde_json::Value {
    let n = m.leaves.len();
    let (mut max, mut min, mut mean) = (
        vec![0.0f32; width],
        vec![0.0f32; width],
        vec![0.0f32; width],
    );
    if n > 0 {
        max = vec![f32::MIN; width];
        min = vec![f32::MAX; width];
        for leaf in &m.leaves {
            for (i, &v) in leaf.iter().enumerate() {
                max[i] = max[i].max(v);
                min[i] = min[i].min(v);
                mean[i] += v / n as f32;
            }
        }
    }
    json!({ "roots": m.roots, "leaves": n, "max": max, "min": min, "mean": mean })
}

fn probe_card(name: &str, probes: &[&Probe], budget: usize, width: usize) -> serde_json::Value {
    let started = Instant::now();
    let Some(card) = cantrip_engine::card(name) else {
        return json!({ "name": name, "probes": {}, "errors": { "all": "no such card" }, "ms": 0 });
    };
    let mut out = serde_json::Map::new();
    let mut errors = serde_json::Map::new();
    for probe in probes {
        let result = catch_unwind(AssertUnwindSafe(|| -> Result<serde_json::Value, String> {
            let mut game = position(probe, name)?;
            let m = measure(
                &mut game,
                card,
                budget,
                matches!(probe.name, "empty" | "board"),
            )?;
            Ok(summary(&m, width))
        }));
        match result {
            Ok(Ok(v)) => {
                out.insert(probe.name.into(), v);
            }
            Ok(Err(e)) => {
                errors.insert(probe.name.into(), e.into());
            }
            Err(p) => {
                let why = p
                    .downcast_ref::<String>()
                    .cloned()
                    .or_else(|| p.downcast_ref::<&str>().map(|s| s.to_string()));
                errors.insert(
                    probe.name.into(),
                    format!("panic: {}", why.unwrap_or_default()).into(),
                );
            }
        }
    }
    json!({ "name": name, "probes": out, "errors": errors, "ms": started.elapsed().as_millis() as u64 })
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let arg = |key: &str| {
        args.iter()
            .position(|a| a == key)
            .and_then(|i| args.get(i + 1))
            .cloned()
    };
    let names_path = arg("--names").expect("--names FILE");
    let out_path = arg("--out").expect("--out FILE");
    let threads: usize = arg("--threads").map_or(8, |t| t.parse().expect("--threads N"));
    let budget: usize = arg("--budget").map_or(24, |t| t.parse().expect("--budget N"));
    let wanted: Option<Vec<String>> =
        arg("--probes").map(|p| p.split(',').map(str::to_string).collect());
    let probes: Vec<&'static Probe> = PROBES
        .iter()
        .filter(|p| {
            wanted
                .as_ref()
                .is_none_or(|w| w.iter().any(|n| n == p.name))
        })
        .collect();
    let names: Vec<String> =
        std::io::BufReader::new(std::fs::File::open(&names_path).expect("names file"))
            .lines()
            .map_while(Result::ok)
            .map(|l| l.trim().to_string())
            .filter(|l| !l.is_empty())
            .collect();
    let width = feature_names().len();
    // Quiet the panics a card's probe may raise: they are caught and reported per probe.
    std::panic::set_hook(Box::new(|_| {}));
    let mut out = std::io::BufWriter::new(std::fs::File::create(&out_path).expect("out file"));
    let probe_names: Vec<&str> = probes.iter().map(|p| p.name).collect();
    writeln!(
        out,
        "{}",
        json!({ "features": feature_names(), "probes": probe_names, "budget": budget })
    )
    .unwrap();
    let total = names.len();
    let queue = Arc::new(Mutex::new(
        names
            .into_iter()
            .enumerate()
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect::<Vec<_>>(),
    ));
    let (tx, rx) = channel();
    let mut workers = Vec::new();
    for _ in 0..threads {
        let queue = Arc::clone(&queue);
        let tx = tx.clone();
        let probes = probes.clone();
        workers.push(
            std::thread::Builder::new()
                .stack_size(STACK_SIZE)
                .spawn(move || {
                    loop {
                        let Some((i, name)) = queue.lock().unwrap().pop() else {
                            break;
                        };
                        let v = probe_card(&name, &probes, budget, width);
                        if tx.send((i, v)).is_err() {
                            break;
                        }
                    }
                })
                .expect("starts"),
        );
    }
    drop(tx);
    let started = Instant::now();
    let mut done = 0;
    let mut pending: HashMap<usize, serde_json::Value> = HashMap::new();
    let mut next = 0;
    for (i, v) in rx {
        pending.insert(i, v);
        while let Some(v) = pending.remove(&next) {
            writeln!(out, "{v}").unwrap();
            next += 1;
        }
        done += 1;
        if done % 100 == 0 || done == total {
            eprintln!(
                "{done}/{total} cards, {:.0} s",
                started.elapsed().as_secs_f64()
            );
            out.flush().unwrap();
        }
    }
    for w in workers {
        let _ = w.join();
    }
}

fn debug() -> bool {
    std::env::var_os("CANTRIP_FINGERPRINT_DEBUG").is_some()
}

fn kind_label(kind: &RequestKind) -> String {
    let d = format!("{kind:?}");
    d.chars().take(60).collect()
}

/// Whether an answer gives up the spell or ability being announced.
fn cancels(answer: &Answer) -> bool {
    matches!(answer, Answer::Cancel)
        || matches!(answer, Answer::Act(a) if a.kind == ActionKind::CancelCasting)
}
