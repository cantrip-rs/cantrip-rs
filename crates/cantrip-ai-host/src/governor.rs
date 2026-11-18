//! Self-play's governor (docs/ai-host.md, "The governor"): how many games run at once and when the next one
//! starts, never how a game is played. A game is dealt and searched by its index alone, so what the governor decides
//! changes how fast the games come, not what they are.
//!
//! - **Admission.** A lane asks before it starts a game (`Governor::admit`) and waits while there is no room: room is
//!   this process's private bytes plus what the games in flight may still grow by and one more game under the budget
//!   (`Govern::budget`), and the system's available physical memory and commit headroom each above their reserve with
//!   that much taken from them (`Govern::reserve`, `commit_reserve`). What a game takes is learnt as the run goes
//!   (`Governor::estimate`: the games' share of what the process grew by, less the lanes' models, `Governor::loading`),
//!   starting from a guess (`Govern::per_game`). With no game running a lane is always let in. A lane that waits is
//!   idled (`Evaluator::idle`): no client of a batching evaluator meanwhile.
//! - **Scaling** (`Scaler`, with `Govern::scale`): a feedback controller on the lanes allowed to play, `--lanes` its
//!   ceiling. It starts with a few and adds more each window while the evaluations a second keep rising by enough for
//!   each lane added, and stops at the first limiter: a throughput plateau (the GPU's batches full, or the CPU's lanes
//!   saturated), the system's CPU busy past a ceiling, or memory admission deferring. A later drop backs it off. A lane
//!   switched off finishes its game and then idles. The gain is a lane's, as a share of an average lane's throughput
//!   (`Scale::min_gain`), not the throughput's own rise, which shrinks with the lanes even when they scale perfectly.
//! - **Graceful stop.** Available memory or commit headroom under the floor for `Govern::floor_ticks` readings in a row
//!   (`Govern::floor`), or an interrupt (Ctrl-C, a console close, SIGINT or SIGTERM: `install_signal_handlers`), stops
//!   admission; the games in flight finish, except that while memory stays under the floor the youngest is stopped at
//!   its current decision, the next only once it has let go (a second interrupt stops them all, a third exits at once).
//!   The lanes then close their shards, and the run's report says why it stopped. A stop file (`Govern::stop_file`,
//!   `cantrip-selfplay`'s `OUT/selfplay.stop`) asks the same of a run nothing can interrupt: a process started over SSH
//!   or by a shell that ignores Ctrl-C (Windows passes that on to its children). Its being there is one interrupt; a
//!   file that says `now` is two, the games in flight stopped too. It is read each tick, and one there when the run
//!   starts stops it before any game. A lease (`Govern::lease`) is a file someone else keeps touching: once it is
//!   older than the lease (or gone), the run stops the same way, so a run whose coordinator went away does not play on
//!   forever (the distributed driver's heartbeat).

use std::fmt;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex, MutexGuard};
use std::time::{Duration, Instant};

use serde_json::{Value, json};

/// A gigabyte as the flags count it: 2^30 bytes.
pub const GB: f64 = (1u64 << 30) as f64;

/// `x` GB in bytes.
pub fn gb(x: f64) -> u64 {
    (x.max(0.0) * GB) as u64
}

fn in_gb(bytes: u64) -> f64 {
    bytes as f64 / GB
}

/// What a game stopped by the governor returns from `selfplay::play_with`: not a failure, and never recorded.
pub const STOPPED: &str = "stopped by the governor";

/// The exit code of a run the governor stopped (`cantrip-selfplay`): its finished games are in closed shards, and
/// `--resume` plays the rest.
pub const EXIT_STOPPED: i32 = 3;

/// One reading of memory, in bytes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Memory {
    /// This process's private bytes (Windows' `PrivateUsage`; Linux's anonymous resident and swapped bytes).
    pub private: u64,
    /// The system's available physical memory.
    pub available: u64,
    /// The system's commit limit less what is committed (Windows' `ullAvailPageFile`); on Linux, where commit is not
    /// limited unless overcommit is off (`vm.overcommit_memory` 2), `u64::MAX`.
    pub commit_headroom: u64,
}

/// Where the governor reads memory and CPU: the system (`SystemProbe`), or a test's.
pub trait Probe: Send + Sync {
    /// Memory now; `None` where it cannot be read (no limit then applies).
    fn memory(&self) -> Option<Memory>;
    /// The system's CPU time so far, idle and in all, in any one unit; `None` where it cannot be read. The busy share
    /// over an interval is one less the idle time's growth over the total's.
    fn cpu(&self) -> Option<(u64, u64)> {
        None
    }
}

/// No readings: the governor admits every lane and never stops for memory.
pub struct NoProbe;

impl Probe for NoProbe {
    fn memory(&self) -> Option<Memory> {
        None
    }
}

/// The machine's own readings: `GlobalMemoryStatusEx`, `GetProcessMemoryInfo` and `GetSystemTimes` on Windows,
/// `/proc` on Linux, and nothing elsewhere.
pub struct SystemProbe;

#[cfg(windows)]
impl Probe for SystemProbe {
    fn memory(&self) -> Option<Memory> {
        use windows_sys::Win32::System::ProcessStatus::{GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS, PROCESS_MEMORY_COUNTERS_EX};
        use windows_sys::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};
        use windows_sys::Win32::System::Threading::GetCurrentProcess;
        // SAFETY: both structures are plain data of the size given, and the calls only write into them.
        unsafe {
            let mut status: MEMORYSTATUSEX = std::mem::zeroed();
            status.dwLength = size_of::<MEMORYSTATUSEX>() as u32;
            if GlobalMemoryStatusEx(&mut status) == 0 {
                return None;
            }
            let mut counters: PROCESS_MEMORY_COUNTERS_EX = std::mem::zeroed();
            counters.cb = size_of::<PROCESS_MEMORY_COUNTERS_EX>() as u32;
            if GetProcessMemoryInfo(GetCurrentProcess(), &mut counters as *mut _ as *mut PROCESS_MEMORY_COUNTERS, counters.cb) == 0 {
                return None;
            }
            Some(Memory {
                private: counters.PrivateUsage as u64,
                available: status.ullAvailPhys,
                commit_headroom: status.ullAvailPageFile,
            })
        }
    }

    fn cpu(&self) -> Option<(u64, u64)> {
        use windows_sys::Win32::Foundation::FILETIME;
        use windows_sys::Win32::System::Threading::GetSystemTimes;
        let time = |f: FILETIME| (u64::from(f.dwHighDateTime) << 32) | u64::from(f.dwLowDateTime);
        // SAFETY: three FILETIMEs the call writes into.
        unsafe {
            let (mut idle, mut kernel, mut user) = (std::mem::zeroed(), std::mem::zeroed(), std::mem::zeroed());
            if GetSystemTimes(&mut idle, &mut kernel, &mut user) == 0 {
                return None;
            }
            // Kernel time includes the idle time.
            Some((time(idle), time(kernel) + time(user)))
        }
    }
}

#[cfg(target_os = "linux")]
impl Probe for SystemProbe {
    fn memory(&self) -> Option<Memory> {
        let kb = |text: &str, key: &str| -> Option<u64> {
            text.lines()
                .find_map(|l| l.strip_prefix(key))
                .and_then(|rest| rest.trim().trim_end_matches("kB").trim().parse::<u64>().ok())
                .map(|k| k * 1024)
        };
        let status = std::fs::read_to_string("/proc/self/status").ok()?;
        let info = std::fs::read_to_string("/proc/meminfo").ok()?;
        let private = kb(&status, "RssAnon:")? + kb(&status, "VmSwap:").unwrap_or(0);
        let available = kb(&info, "MemAvailable:")?;
        let strict = std::fs::read_to_string("/proc/sys/vm/overcommit_memory").is_ok_and(|v| v.trim() == "2");
        let commit_headroom = if strict {
            kb(&info, "CommitLimit:")?.saturating_sub(kb(&info, "Committed_AS:")?)
        } else {
            u64::MAX
        };
        Some(Memory { private, available, commit_headroom })
    }

    fn cpu(&self) -> Option<(u64, u64)> {
        let stat = std::fs::read_to_string("/proc/stat").ok()?;
        let line = stat.lines().next()?.strip_prefix("cpu ")?;
        let v: Vec<u64> = line.split_whitespace().take(8).filter_map(|x| x.parse().ok()).collect();
        if v.len() < 5 {
            return None;
        }
        Some((v[3] + v[4], v.iter().sum()))
    }
}

#[cfg(not(any(windows, target_os = "linux")))]
impl Probe for SystemProbe {
    fn memory(&self) -> Option<Memory> {
        None
    }
}

/// The scaling controller's settings (`Scaler`).
#[derive(Clone, Debug)]
pub struct Scale {
    /// Lanes allowed to play at first.
    pub start: usize,
    /// Lanes added at a time while the gains continue.
    pub step: usize,
    /// How long throughput is measured before each decision.
    pub window: Duration,
    /// The least marginal gain worth another lane: the throughput the lanes added since a level at most 0.8 of this
    /// one brought, each, as a share of what the average lane made there (1: they scale linearly; 0: not at all).
    pub min_gain: f64,
    /// The system's busy CPU share past which no lane is added.
    pub cpu_ceiling: f64,
    /// A fall in throughput, as a share, that backs the lanes off when two windows in a row show it.
    pub drop: f64,
    /// Windows held at a plateau before growth is tried again.
    pub reprobe: u32,
}

impl Default for Scale {
    fn default() -> Scale {
        Scale {
            start: 4,
            step: 2,
            window: Duration::from_secs(45),
            min_gain: 0.2,
            cpu_ceiling: 0.95,
            drop: 0.15,
            reprobe: 10,
        }
    }
}

/// What stopped the lanes growing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Limiter {
    /// `--lanes`.
    Ceiling,
    /// Lanes added stopped adding enough throughput: the GPU's batches or the CPU's cores are full.
    Plateau,
    /// The system's CPU busy past `Scale::cpu_ceiling`.
    Cpu,
    /// Memory admission deferred a game.
    Memory,
    /// Throughput fell at a level held, and the lanes were backed off.
    Drop,
}

impl Limiter {
    pub fn name(self) -> &'static str {
        match self {
            Limiter::Ceiling => "ceiling",
            Limiter::Plateau => "throughput plateau",
            Limiter::Cpu => "cpu utilisation",
            Limiter::Memory => "memory",
            Limiter::Drop => "throughput drop",
        }
    }
}

/// One window's measurements, as the scaler reads them.
#[derive(Clone, Copy, Debug, Default)]
pub struct Window {
    pub seconds: f64,
    /// Evaluations a second.
    pub throughput: f64,
    /// Games running, on average over the window.
    pub lanes: f64,
    /// The system's busy CPU share, where known.
    pub cpu: Option<f64>,
    /// Whether memory admission deferred a game in the window.
    pub memory_deferred: bool,
}

/// What the scaler made of a window: the lanes allowed now, the change, and why.
#[derive(Clone, Debug, PartialEq)]
pub struct Decision {
    pub target: usize,
    pub change: i64,
    pub limiter: Option<Limiter>,
    pub reason: String,
}

struct Hold {
    limiter: Limiter,
    windows: u32,
    /// The throughput held at, followed slowly; none until a window at this level is measured.
    reference: Option<f64>,
    /// Windows in a row under the reference by more than `Scale::drop`.
    low: u32,
    /// Windows in a row the CPU or memory limiter was clear.
    clear: u32,
}

/// The feedback controller on the lanes allowed to play (see the module). It grows by `Scale::step` each window while
/// the marginal gain holds; a gain under `Scale::min_gain` is measured again for a second window before it counts, and
/// a level held is watched for a drop over two windows, so one noisy window neither stops growth nor backs it off.
pub struct Scaler {
    cfg: Scale,
    ceiling: usize,
    target: usize,
    /// (lanes allowed, lanes busy, throughput) of each level measured while growing.
    history: Vec<(usize, f64, f64)>,
    /// A first window at this level whose gain was too little.
    doubt: Option<(f64, f64)>,
    hold: Option<Hold>,
    /// The last limiter that stopped growth.
    pub limiter: Option<Limiter>,
}

impl Scaler {
    pub fn new(cfg: Scale, ceiling: usize) -> Scaler {
        let ceiling = ceiling.max(1);
        let target = cfg.start.clamp(1, ceiling);
        Scaler { cfg, ceiling, target, history: Vec::new(), doubt: None, hold: None, limiter: None }
    }

    pub fn target(&self) -> usize {
        self.target
    }

    fn keep(&self, reason: String) -> Decision {
        Decision { target: self.target, change: 0, limiter: self.limiter, reason }
    }

    fn set(&mut self, target: usize, reason: String) -> Decision {
        let target = target.clamp(1, self.ceiling);
        let change = target as i64 - self.target as i64;
        self.target = target;
        Decision { target, change, limiter: self.limiter, reason }
    }

    fn hold_at(&mut self, limiter: Limiter, reason: String) -> Decision {
        self.doubt = None;
        match &mut self.hold {
            Some(h) if h.limiter == limiter => {
                h.windows += 1;
                h.clear = 0;
            }
            _ => self.hold = Some(Hold { limiter, windows: 0, reference: None, low: 0, clear: 0 }),
        }
        self.limiter = Some(limiter);
        self.keep(reason)
    }

    fn grow(&mut self, by: usize, why: String) -> Decision {
        if self.target >= self.ceiling {
            self.hold = Some(Hold { limiter: Limiter::Ceiling, windows: 0, reference: None, low: 0, clear: 0 });
            self.limiter = Some(Limiter::Ceiling);
            return self.keep(format!("{why}; at the ceiling of {} lanes", self.ceiling));
        }
        let to = (self.target + by).min(self.ceiling);
        self.set(to, why)
    }

    /// The marginal gain from `reference`'s level to this one (see `Scale::min_gain`).
    fn gain(reference: (f64, f64), lanes: f64, throughput: f64) -> Option<f64> {
        let (l0, t0) = reference;
        (lanes - l0 >= 0.5 && t0 > 0.0 && l0 > 0.0).then(|| ((throughput - t0) / (lanes - l0)) / (t0 / l0))
    }

    fn reference(&self, lanes: f64) -> Option<(f64, f64)> {
        self.history.iter().rev().find(|(_, l, _)| *l <= lanes * 0.8).or(self.history.first()).map(|&(_, l, t)| (l, t))
    }

    /// The lanes allowed after window `w`.
    pub fn step(&mut self, w: &Window) -> Decision {
        let target = self.target;
        if w.lanes < 0.5 || w.seconds <= 0.0 {
            return self.keep("no game ran".into());
        }
        if w.memory_deferred {
            return self.hold_at(Limiter::Memory, "memory admission deferred a game: no lane added".into());
        }
        if let Some(cpu) = w.cpu.filter(|&c| c > self.cfg.cpu_ceiling) {
            return self.hold_at(
                Limiter::Cpu,
                format!("the CPU is {:.0}% busy, over the ceiling of {:.0}%: no lane added", cpu * 100.0, self.cfg.cpu_ceiling * 100.0),
            );
        }
        if let Some(h) = &mut self.hold {
            h.windows += 1;
            match h.limiter {
                Limiter::Memory | Limiter::Cpu => {
                    h.clear += 1;
                    if h.clear < 2 {
                        let name = h.limiter.name();
                        return self.keep(format!("{name} clear for a window: holding one more"));
                    }
                    let name = h.limiter.name();
                    self.hold = None;
                    self.history = vec![(target, w.lanes, w.throughput)];
                    return self.grow(self.cfg.step, format!("{name} clear for two windows: growing again"));
                }
                Limiter::Plateau | Limiter::Drop | Limiter::Ceiling => {
                    match h.reference {
                        None => h.reference = Some(w.throughput),
                        Some(r) if w.throughput < r * (1.0 - self.cfg.drop) => {
                            h.low += 1;
                            if h.low >= 2 && target > 1 {
                                let fell = 1.0 - w.throughput / r;
                                h.reference = None;
                                h.low = 0;
                                h.windows = 0;
                                h.limiter = Limiter::Drop;
                                self.limiter = Some(Limiter::Drop);
                                let back = target.saturating_sub(self.cfg.step).max(1);
                                return self.set(
                                    back,
                                    format!("throughput {:.0}% under the level held for two windows: backing off", fell * 100.0),
                                );
                            }
                            return self.keep(format!("throughput {:.0}% under the level held: watching", (1.0 - w.throughput / r) * 100.0));
                        }
                        Some(r) => {
                            h.low = 0;
                            h.reference = Some(0.8 * r + 0.2 * w.throughput);
                        }
                    }
                    if h.limiter != Limiter::Ceiling && h.windows >= self.cfg.reprobe && target < self.ceiling {
                        let windows = h.windows;
                        self.hold = None;
                        self.history = vec![(target, w.lanes, w.throughput)];
                        let by = self.cfg.step.max(target.div_ceil(8));
                        return self.grow(by, format!("held {windows} windows: probing for more"));
                    }
                    let name = h.limiter.name();
                    return self.keep(format!("holding ({name})"));
                }
            }
        }
        // Growing: only on a level the lanes actually fill (a tail of the run, or lanes still starting, is no measure).
        if w.lanes < target as f64 * 0.9 - 0.5 {
            return self.keep(format!("{:.1} of {target} lanes busy: no lane added until they are", w.lanes));
        }
        let mut level = (w.lanes, w.throughput);
        let mut gain = self.reference(w.lanes).and_then(|r| Self::gain(r, level.0, level.1));
        if gain.is_some_and(|g| g < self.cfg.min_gain) {
            let Some(first) = self.doubt.take() else {
                self.doubt = Some(level);
                return self.keep(format!(
                    "marginal gain {:.2} a lane, under {:.2}: measuring this level again",
                    gain.unwrap_or_default(),
                    self.cfg.min_gain
                ));
            };
            level = ((first.0 + w.lanes) / 2.0, (first.1 + w.throughput) / 2.0);
            gain = self.reference(level.0).and_then(|r| Self::gain(r, level.0, level.1));
            if let Some(g) = gain.filter(|&g| g < self.cfg.min_gain) {
                // Held at the last level whose gain paid (or where a probe began).
                self.limiter = Some(Limiter::Plateau);
                self.hold = Some(Hold { limiter: Limiter::Plateau, windows: 0, reference: None, low: 0, clear: 0 });
                let back = self.history.last().map_or(target, |&(allowed, _, _)| allowed.max(1)).min(target);
                return self.set(
                    back,
                    format!("marginal gain {g:.2} a lane over two windows, under {:.2}: holding at the last level that paid", self.cfg.min_gain),
                );
            }
        }
        self.doubt = None;
        self.history.push((target, level.0, level.1));
        let why = match gain {
            Some(g) => format!("marginal gain {g:.2} a lane"),
            None => "first measure".into(),
        };
        self.grow(self.cfg.step, why)
    }
}

/// The governor's settings.
#[derive(Clone)]
pub struct Govern {
    /// This process's private bytes may not pass this with one more game (none: no cap).
    pub budget: Option<u64>,
    /// Available physical memory kept free with one more game.
    pub reserve: u64,
    /// Commit headroom kept free with one more game.
    pub commit_reserve: u64,
    /// Available memory or commit headroom under this for `floor_ticks` ticks in a row stops the run (0: never).
    pub floor: u64,
    pub floor_ticks: u32,
    /// The guess at a game's memory until games teach it.
    pub per_game: u64,
    /// The scaling controller; none: every lane up to `--lanes` may play.
    pub scale: Option<Scale>,
    /// How often memory is read and the stop conditions checked.
    pub tick: Duration,
    pub probe: Arc<dyn Probe>,
    /// Interrupts counted: one stops admission, two stop the games in flight too. A test's to set; with `signals`,
    /// Ctrl-C and SIGTERM count here too (`install_signal_handlers`).
    pub interrupts: Arc<AtomicU32>,
    pub signals: bool,
    /// Whether decisions and stops are logged to stderr.
    pub log: bool,
    /// A file whose being there stops the run as an interrupt does, and whose saying `now` stops the games in flight
    /// too (see the module); none: not looked for.
    pub stop_file: Option<std::path::PathBuf>,
    /// A heartbeat file and how old it may grow: older (or gone), the run stops gracefully (see the module).
    pub lease: Option<(std::path::PathBuf, Duration)>,
}

impl fmt::Debug for Govern {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Govern")
            .field("budget", &self.budget)
            .field("reserve", &self.reserve)
            .field("commit_reserve", &self.commit_reserve)
            .field("floor", &self.floor)
            .field("floor_ticks", &self.floor_ticks)
            .field("per_game", &self.per_game)
            .field("scale", &self.scale)
            .field("tick", &self.tick)
            .field("stop_file", &self.stop_file)
            .field("lease", &self.lease)
            .finish_non_exhaustive()
    }
}

impl Default for Govern {
    /// Off: no readings, no limits, every lane playing (a library caller's and a test's run).
    fn default() -> Govern {
        Govern {
            budget: None,
            reserve: 0,
            commit_reserve: 0,
            floor: 0,
            floor_ticks: 4,
            per_game: gb(1.0),
            scale: None,
            tick: Duration::from_millis(250),
            probe: Arc::new(NoProbe),
            interrupts: Arc::new(AtomicU32::new(0)),
            signals: false,
            log: false,
            stop_file: None,
            lease: None,
        }
    }
}

impl Govern {
    /// `cantrip-selfplay`'s defaults: the machine's readings, 6 GB kept available and of commit, a 2 GB floor, a 1 GB
    /// guess a game, scaling on, interrupts heard.
    pub fn system() -> Govern {
        Govern {
            reserve: gb(6.0),
            commit_reserve: gb(6.0),
            floor: gb(2.0),
            scale: Some(Scale::default()),
            probe: Arc::new(SystemProbe),
            signals: true,
            log: true,
            ..Govern::default()
        }
    }
}

/// Whether one more game needing `need` bytes fits `m` under `g`'s limits: `Err` says which limit it would pass.
pub fn room(g: &Govern, m: Memory, need: u64) -> Result<(), String> {
    if let Some(budget) = g.budget
        && m.private.saturating_add(need) > budget
    {
        return Err(format!(
            "private {:.1} GB with {:.1} GB more would pass the budget of {:.1} GB",
            in_gb(m.private),
            in_gb(need),
            in_gb(budget)
        ));
    }
    if m.available.saturating_sub(need) < g.reserve || m.available < need {
        return Err(format!(
            "{:.1} GB available less {:.1} GB would be under the reserve of {:.1} GB",
            in_gb(m.available),
            in_gb(need),
            in_gb(g.reserve)
        ));
    }
    if m.commit_headroom.saturating_sub(need) < g.commit_reserve || m.commit_headroom < need {
        return Err(format!(
            "{:.1} GB of commit headroom less {:.1} GB would be under the reserve of {:.1} GB",
            in_gb(m.commit_headroom),
            in_gb(need),
            in_gb(g.commit_reserve)
        ));
    }
    Ok(())
}

/// The work done so far, counted by the games as they search (`selfplay::Players::meter`).
#[derive(Default)]
pub struct Meter {
    pub evaluations: AtomicU64,
    pub searched: AtomicU64,
}

#[derive(Clone, Copy)]
struct Slot {
    running: bool,
    started: Instant,
    /// What this game was seen to take, summed over the ticks it was read at (`Governor::tick`), and those ticks.
    sum: f64,
    seen: u32,
    aborted: bool,
}

impl Slot {
    fn idle(now: Instant) -> Slot {
        Slot { running: false, started: now, sum: 0.0, seen: 0, aborted: false }
    }

    /// What this game takes, as read so far: the mean of its readings.
    fn taking(&self) -> u64 {
        if self.seen == 0 { 0 } else { (self.sum / f64::from(self.seen)) as u64 }
    }
}

struct State {
    target: usize,
    slots: Vec<Slot>,
    running: usize,
    estimate: u64,
    learned: u64,
    /// The process's private bytes with no game running (the least seen), and the lanes with models loaded then.
    base: Option<(u64, usize)>,
    /// Lanes with their models loaded (`Governor::loading`), and what loading them took, on average over `loads`.
    loaded: usize,
    lane_cost: f64,
    loads: u32,
    /// Whether a game started or ended since the last tick: a tick then reads no game (memory a game just let go of
    /// may not be returned yet).
    changed: bool,
    /// Ticks in a row memory was under the floor.
    under_floor: u32,
    stop: Option<String>,
    /// No game is left to start (all dealt, or the time limit passed).
    closed: bool,
    aborted: u64,
    last_defer_log: Option<Instant>,
    deferred: u64,
    deferred_seconds: f64,
    window_deferred: bool,
    peak_private: u64,
    min_available: Option<u64>,
    min_headroom: Option<u64>,
    /// Lanes running: sampled each tick (min, max), and integrated over time (mean), from the first game to the close.
    lanes_min: Option<usize>,
    lanes_max: usize,
    lanes_area: f64,
    lanes_seconds: f64,
    window_area: f64,
    accounted: Option<Instant>,
    scaler: Option<Scaler>,
    window_start: Option<Instant>,
    window_evaluations: u64,
    window_searched: u64,
    window_cpu: Option<(u64, u64)>,
    windows: Vec<Value>,
}

impl State {
    /// A reading with no game running: the base what the games take is read from.
    fn note_base(&mut self, private: u64) {
        if self.base.is_none_or(|(b, _)| private < b) {
            self.base = Some((private, self.loaded));
        }
    }

    fn observe(&mut self, m: Memory) {
        self.peak_private = self.peak_private.max(m.private);
        self.min_available = Some(self.min_available.map_or(m.available, |x| x.min(m.available)));
        self.min_headroom = Some(self.min_headroom.map_or(m.commit_headroom, |x| x.min(m.commit_headroom)));
    }

    /// The lanes' time integral up to `now`, while games remain to start.
    fn account(&mut self, now: Instant) {
        if let Some(at) = self.accounted {
            let dt = now.saturating_duration_since(at).as_secs_f64();
            if !self.closed && self.stop.is_none() {
                self.lanes_area += self.running as f64 * dt;
                self.lanes_seconds += dt;
            }
            self.window_area += self.running as f64 * dt;
        }
        if self.accounted.is_some() || self.running > 0 {
            self.accounted = Some(now);
        }
    }
}

static SIGNALLED: AtomicU32 = AtomicU32::new(0);
static CLOSED: AtomicBool = AtomicBool::new(false);

/// What a stop file asks (`Govern::stop_file`): 0 nothing (no file), 1 a graceful stop, 2 the games in flight stopped
/// too (the file says `now`).
pub fn stop_file_level(path: &std::path::Path) -> u32 {
    match std::fs::read_to_string(path) {
        Ok(text) if text.trim().eq_ignore_ascii_case("now") => 2,
        Ok(_) => 1,
        // There but not readable this moment (being written): a stop all the same.
        Err(e) if e.kind() != std::io::ErrorKind::NotFound => u32::from(path.exists()),
        Err(_) => 0,
    }
}

/// Whether a lease has run out (`Govern::lease`): its file older than `lease`, or gone. Why, if it has.
pub fn lease_expired(path: &std::path::Path, lease: Duration) -> Option<String> {
    match std::fs::metadata(path).and_then(|m| m.modified()) {
        Ok(t) => {
            let age = t.elapsed().unwrap_or_default();
            (age > lease).then(|| format!("lease {} not renewed for {:.0} s", path.display(), age.as_secs_f64()))
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Some(format!("lease {} gone", path.display())),
        // Being replaced this moment: not expired.
        Err(_) => None,
    }
}

/// Ctrl-C, Ctrl-Break and a console's close, logoff or shutdown on Windows, SIGINT and SIGTERM elsewhere, counted as
/// interrupts of every governor with `Govern::signals`: one stops admission, two stop the games in flight too, and a
/// third exits at once (130). A console's close or SIGTERM counts two: the games in flight are stopped at once, and on
/// a close the handler waits for the shards to close (`mark_closed`) before Windows ends the process.
pub fn install_signal_handlers() {
    #[cfg(windows)]
    {
        use windows_sys::Win32::System::Console::{
            CTRL_BREAK_EVENT, CTRL_C_EVENT, CTRL_CLOSE_EVENT, CTRL_LOGOFF_EVENT, CTRL_SHUTDOWN_EVENT, SetConsoleCtrlHandler,
        };
        unsafe extern "system" fn handler(kind: u32) -> windows_sys::core::BOOL {
            match kind {
                CTRL_C_EVENT | CTRL_BREAK_EVENT => {
                    SIGNALLED.fetch_add(1, Ordering::SeqCst);
                    1
                }
                CTRL_CLOSE_EVENT | CTRL_LOGOFF_EVENT | CTRL_SHUTDOWN_EVENT => {
                    SIGNALLED.fetch_add(2, Ordering::SeqCst);
                    let until = Instant::now() + Duration::from_millis(4500);
                    while !CLOSED.load(Ordering::SeqCst) && Instant::now() < until {
                        std::thread::sleep(Duration::from_millis(50));
                    }
                    1
                }
                _ => 0,
            }
        }
        // SAFETY: the handler only touches atomics and sleeps.
        if unsafe { SetConsoleCtrlHandler(Some(handler), 1) } == 0 {
            eprintln!(
                "selfplay: Ctrl-C cannot be heard ({}): an interrupt ends the run without closing its shards (--resume recovers them)",
                std::io::Error::last_os_error()
            );
        }
    }
    #[cfg(unix)]
    {
        // SIGINT is an interrupt; SIGTERM, as a console's close is, stops the games in flight too.
        extern "C" fn handler(signal: libc::c_int) {
            SIGNALLED.fetch_add(if signal == libc::SIGTERM { 2 } else { 1 }, Ordering::SeqCst);
        }
        // SAFETY: the handler only adds to an atomic, which is async-signal-safe.
        unsafe {
            for signal in [libc::SIGINT, libc::SIGTERM] {
                if libc::signal(signal, handler as extern "C" fn(libc::c_int) as libc::sighandler_t) == libc::SIG_ERR {
                    eprintln!("selfplay: signal {signal} cannot be heard: {}", std::io::Error::last_os_error());
                }
            }
        }
    }
}

/// Whether process `pid` is running (a run's lock, `selfplay::LOCK`). Where it cannot be told, it is taken to be.
pub fn process_alive(pid: u32) -> bool {
    #[cfg(windows)]
    {
        use windows_sys::Win32::Foundation::{CloseHandle, ERROR_ACCESS_DENIED, GetLastError, STILL_ACTIVE};
        use windows_sys::Win32::System::Threading::{GetExitCodeProcess, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION};
        // SAFETY: a handle opened here, read and closed.
        unsafe {
            let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
            if handle.is_null() {
                return GetLastError() == ERROR_ACCESS_DENIED;
            }
            let mut code = 0u32;
            let ok = GetExitCodeProcess(handle, &mut code) != 0;
            CloseHandle(handle);
            !ok || code == STILL_ACTIVE as u32
        }
    }
    #[cfg(unix)]
    {
        // SAFETY: signal 0 only asks whether the process exists.
        unsafe { libc::kill(pid as libc::pid_t, 0) == 0 || std::io::Error::last_os_error().raw_os_error() == Some(libc::EPERM) }
    }
    #[cfg(not(any(windows, unix)))]
    {
        let _ = pid;
        true
    }
}

/// Says the shards are closed, so a console close's handler may let the process end.
pub fn mark_closed() {
    CLOSED.store(true, Ordering::SeqCst);
}

enum Admission<'g> {
    Start(Ticket<'g>),
    /// Not now: memory has no room (`memory`), or the lane is switched off or busy.
    Wait { memory: bool },
    Stop,
}

/// A game admitted: the lane's slot until it is dropped. `finish(true)` says the game ran to its end, so its memory is
/// learnt from; dropped otherwise (stopped, or no game after all), it is not.
pub struct Ticket<'g> {
    governor: &'g Governor,
    pub lane: usize,
    completed: bool,
}

impl Ticket<'_> {
    /// The flag that stops this game at its next decision.
    pub fn abort(&self) -> &AtomicBool {
        &self.governor.aborts[self.lane]
    }

    pub fn finish(mut self, completed: bool) {
        self.completed = completed;
    }
}

impl Drop for Ticket<'_> {
    fn drop(&mut self) {
        self.governor.end(self.lane, self.completed);
    }
}

/// A run's governor (see the module): the lanes ask it before each game, and its monitor (`Governor::monitor`) reads
/// memory each tick.
pub struct Governor {
    cfg: Govern,
    lanes: usize,
    started: Instant,
    deadline: Option<Instant>,
    state: Mutex<State>,
    wake: Condvar,
    aborts: Vec<AtomicBool>,
    pub meter: Meter,
    finished: AtomicBool,
}

impl Governor {
    /// A governor for `lanes` lanes (the ceiling), admitting no game after `limit` from now.
    pub fn new(cfg: Govern, lanes: usize, limit: Option<Duration>) -> Governor {
        let lanes = lanes.max(1);
        let now = Instant::now();
        let scaler = cfg.scale.clone().map(|s| Scaler::new(s, lanes));
        let target = scaler.as_ref().map_or(lanes, Scaler::target);
        let slot = Slot::idle(now);
        // A stop file there before the first game: no game is started at all.
        let stop = cfg
            .stop_file
            .as_deref()
            .filter(|p| stop_file_level(p) > 0)
            .map(|p| format!("stop file {}", p.display()));
        if let Some(why) = &stop
            && cfg.log
        {
            eprintln!("selfplay: stopping: {why} was there at the start; remove it to play");
        }
        Governor {
            lanes,
            started: now,
            deadline: limit.map(|l| now + l),
            state: Mutex::new(State {
                target,
                slots: vec![slot; lanes],
                running: 0,
                estimate: cfg.per_game,
                learned: 0,
                base: None,
                loaded: 0,
                lane_cost: 0.0,
                loads: 0,
                changed: false,
                under_floor: 0,
                stop,
                closed: false,
                aborted: 0,
                last_defer_log: None,
                deferred: 0,
                deferred_seconds: 0.0,
                window_deferred: false,
                peak_private: 0,
                min_available: None,
                min_headroom: None,
                lanes_min: None,
                lanes_max: 0,
                lanes_area: 0.0,
                lanes_seconds: 0.0,
                window_area: 0.0,
                accounted: None,
                scaler,
                window_start: None,
                window_evaluations: 0,
                window_searched: 0,
                window_cpu: None,
                windows: Vec::new(),
            }),
            wake: Condvar::new(),
            aborts: (0..lanes).map(|_| AtomicBool::new(false)).collect(),
            meter: Meter::default(),
            finished: AtomicBool::new(false),
            cfg,
        }
    }

    fn lock(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn say(&self, line: &str) {
        if self.cfg.log {
            eprintln!("selfplay: {line}");
        }
    }

    /// Why the run stopped, if it did.
    pub fn stopped(&self) -> Option<String> {
        self.lock().stop.clone()
    }

    /// The lanes allowed to play now.
    pub fn target(&self) -> usize {
        self.lock().target
    }

    /// The flag that stops lane `lane`'s game at its next decision.
    pub fn abort_flag(&self, lane: usize) -> &AtomicBool {
        &self.aborts[lane]
    }

    /// The memory a game is reckoned to take, in bytes.
    pub fn estimate(&self) -> u64 {
        self.lock().estimate
    }

    /// No game is left to start: every waiting lane is let go.
    pub fn close(&self) {
        let mut s = self.lock();
        s.account(Instant::now());
        s.closed = true;
        drop(s);
        self.wake.notify_all();
    }

    /// Stops admission, as an interrupt or the floor does.
    pub fn stop(&self, why: &str) {
        let mut s = self.lock();
        if s.stop.is_none() {
            s.account(Instant::now());
            s.stop = Some(why.to_string());
            drop(s);
            self.say(&format!("stopping: {why}; no game is started from here"));
        }
        self.wake.notify_all();
    }

    fn admissible(&self, s: &State) -> bool {
        s.stop.is_none() && !s.closed && self.deadline.is_none_or(|d| Instant::now() < d)
    }

    /// Whether lane `lane` may start a game now.
    fn try_admit_locked<'g>(&'g self, s: &mut State, lane: usize) -> Admission<'g> {
        if !self.admissible(s) {
            return Admission::Stop;
        }
        if lane >= s.target || s.slots[lane].running {
            return Admission::Wait { memory: false };
        }
        let reading = self.cfg.probe.memory();
        if let Some(m) = reading {
            s.observe(m);
            if s.running == 0 {
                s.note_base(m.private);
            }
        }
        if s.running > 0
            && let Some(m) = reading
        {
            let pending: u64 = s.slots.iter().filter(|x| x.running).map(|x| s.estimate.saturating_sub(x.taking())).sum();
            if let Err(why) = room(&self.cfg, m, pending + s.estimate) {
                s.window_deferred = true;
                let now = Instant::now();
                if s.last_defer_log.is_none_or(|t| now.duration_since(t) >= Duration::from_secs(60)) {
                    s.last_defer_log = Some(now);
                    self.say(&format!(
                        "admission deferring with {} games running ({:.2} GB a game reckoned): {why}",
                        s.running,
                        in_gb(s.estimate)
                    ));
                }
                return Admission::Wait { memory: true };
            }
        }
        let now = Instant::now();
        s.account(now);
        s.slots[lane] = Slot { running: true, ..Slot::idle(now) };
        s.running += 1;
        s.changed = true;
        if s.accounted.is_none() {
            s.accounted = Some(now);
        }
        if s.window_start.is_none() {
            s.window_start = Some(now);
            s.window_cpu = self.cfg.probe.cpu();
        }
        self.aborts[lane].store(false, Ordering::SeqCst);
        Admission::Start(Ticket { governor: self, lane, completed: false })
    }

    /// One try at admission for lane `lane`, not waiting: `Ok(None)` if it would wait, `Err(())` if the run takes no
    /// more games.
    pub fn try_admit(&self, lane: usize) -> Result<Option<Ticket<'_>>, ()> {
        let mut s = self.lock();
        match self.try_admit_locked(&mut s, lane) {
            Admission::Start(t) => Ok(Some(t)),
            Admission::Wait { .. } => Ok(None),
            Admission::Stop => Err(()),
        }
    }

    /// Waits until lane `lane` may start a game (see the module); `None` once the run takes no more. `idle` is called
    /// once, outside the lock, when the lane has to wait: a lane waiting for admission or switched off is no client of a
    /// batching evaluator (`Evaluator::idle`).
    pub fn admit(&self, lane: usize, mut idle: impl FnMut()) -> Option<Ticket<'_>> {
        let mut s = self.lock();
        let mut waited: Option<Instant> = None;
        let mut idled = false;
        loop {
            let ticket = match self.try_admit_locked(&mut s, lane) {
                Admission::Start(t) => Some(t),
                Admission::Stop => None,
                Admission::Wait { memory } => {
                    if memory && waited.is_none() {
                        waited = Some(Instant::now());
                        s.deferred += 1;
                    }
                    if !idled {
                        idled = true;
                        drop(s);
                        idle();
                        s = self.lock();
                        continue;
                    }
                    s = self.wake.wait_timeout(s, self.cfg.tick).map(|(g, _)| g).unwrap_or_else(|e| e.into_inner().0);
                    continue;
                }
            };
            if let Some(w) = waited {
                s.deferred_seconds += w.elapsed().as_secs_f64();
            }
            return ticket;
        }
    }

    fn end(&self, lane: usize, completed: bool) {
        let mut s = self.lock();
        let now = Instant::now();
        s.account(now);
        let slot = s.slots[lane];
        if !slot.running {
            return;
        }
        s.slots[lane].running = false;
        s.running -= 1;
        s.changed = true;
        if completed {
            // A game that finished as it was being stopped is a game finished, not one stopped.
            if slot.seen > 0 {
                // Half the way up to a game seen to take more, a tenth of the way down to one that takes less.
                let (e, g) = (s.estimate as f64, slot.taking() as f64);
                s.estimate = (e + (g - e) * if g > e { 0.5 } else { 0.1 }) as u64;
                s.learned += 1;
            }
        } else if slot.aborted {
            s.aborted += 1;
        }
        drop(s);
        self.wake.notify_all();
    }

    /// Runs `load` (a lane loading its models) and learns what a lane's models take, so that what the games take is
    /// read apart from it.
    pub fn loading<R>(&self, load: impl FnOnce() -> R) -> R {
        let before = self.cfg.probe.memory();
        let r = load();
        let after = self.cfg.probe.memory();
        let mut s = self.lock();
        s.loaded += 1;
        if let (Some(a), Some(b)) = (before, after) {
            s.loads += 1;
            let took = b.private.saturating_sub(a.private) as f64;
            s.lane_cost += (took - s.lane_cost) / f64::from(s.loads);
        }
        r
    }

    /// One tick of the monitor at `now`: memory read, the floor and interrupts checked, a game stopped if memory has
    /// been under the floor for `Govern::floor_ticks` ticks, and the scaler's window closed when it is due.
    pub fn tick(&self, now: Instant) {
        let reading = self.cfg.probe.memory();
        let mut s = self.lock();
        s.account(now);
        let changed = std::mem::replace(&mut s.changed, false);
        if let Some(m) = reading {
            s.observe(m);
            if s.running == 0 {
                s.note_base(m.private);
            } else if let Some((base, base_loaded)) = s.base
                && !changed
            {
                // The games' share of what the process grew by since a moment with none running, less the models of
                // the lanes loaded since; read only when no game started or ended since the last tick.
                let lanes = s.loaded.saturating_sub(base_loaded) as f64 * s.lane_cost;
                let each = (m.private.saturating_sub(base) as f64 - lanes).max(0.0) / s.running as f64;
                for slot in s.slots.iter_mut().filter(|x| x.running) {
                    slot.sum += each;
                    slot.seen += 1;
                }
            }
            let under = self.cfg.floor > 0 && (m.available < self.cfg.floor || m.commit_headroom < self.cfg.floor);
            s.under_floor = if under { s.under_floor + 1 } else { 0 };
            if s.under_floor >= self.cfg.floor_ticks.max(1) {
                if s.stop.is_none() {
                    let why = format!(
                        "memory under the floor of {:.1} GB ({:.1} GB available, {:.1} GB of commit headroom)",
                        in_gb(self.cfg.floor),
                        in_gb(m.available),
                        in_gb(m.commit_headroom)
                    );
                    s.stop = Some(why.clone());
                    self.say(&format!("stopping: {why}; no game is started from here"));
                }
                // The youngest game in flight, stopped at its next decision: one at a time while memory stays under,
                // the next once the last has let go.
                let stopping = s.slots.iter().any(|x| x.running && x.aborted);
                let youngest = (0..s.slots.len())
                    .filter(|&i| s.slots[i].running && !s.slots[i].aborted)
                    .max_by_key(|&i| s.slots[i].started);
                if let Some(i) = youngest.filter(|_| !stopping) {
                    s.slots[i].aborted = true;
                    self.aborts[i].store(true, Ordering::SeqCst);
                    self.say(&format!("memory still under the floor: stopping lane {i}'s game, dropped and played again on --resume"));
                }
            }
        }
        let interrupts = self.cfg.interrupts.load(Ordering::SeqCst) + if self.cfg.signals { SIGNALLED.load(Ordering::SeqCst) } else { 0 };
        let filed = self.cfg.stop_file.as_deref().map_or(0, stop_file_level);
        if interrupts >= 1 && s.stop.is_none() {
            let why = "interrupted".to_string();
            s.stop = Some(why);
            self.say(&format!(
                "interrupted: no game is started from here; the {} in flight finish (interrupt again to stop them too)",
                s.running
            ));
        }
        if filed >= 1 && s.stop.is_none() {
            let why = format!("stop file {}", self.cfg.stop_file.as_deref().map(|p| p.display().to_string()).unwrap_or_default());
            self.say(&format!(
                "{why}: no game is started from here; the {} in flight finish (write `now` in it to stop them too)",
                s.running
            ));
            s.stop = Some(why);
        }
        if let Some((path, lease)) = &self.cfg.lease
            && s.stop.is_none()
            && let Some(why) = lease_expired(path, *lease)
        {
            self.say(&format!("{why}: no game is started from here; the {} in flight finish", s.running));
            s.stop = Some(why);
        }
        // A stop file's `now` stops the games in flight as a second interrupt does (never a third's exit).
        let interrupts = interrupts.max(filed);
        if interrupts >= 3 {
            self.say("interrupted a third time: exiting at once (the shards open now are recovered on --resume)");
            std::process::exit(130);
        }
        if interrupts >= 2 {
            let mut stopped = 0;
            for i in 0..s.slots.len() {
                if s.slots[i].running && !s.slots[i].aborted {
                    s.slots[i].aborted = true;
                    self.aborts[i].store(true, Ordering::SeqCst);
                    stopped += 1;
                }
            }
            if stopped > 0 {
                self.say(&format!("interrupted again: stopping the {stopped} games in flight"));
            }
        }
        if s.accounted.is_some() && !s.closed && s.stop.is_none() {
            let r = s.running;
            s.lanes_min = Some(s.lanes_min.map_or(r, |m| m.min(r)));
            s.lanes_max = s.lanes_max.max(r);
        }
        self.close_window(&mut s, now);
        drop(s);
        self.wake.notify_all();
    }

    fn close_window(&self, s: &mut State, now: Instant) {
        let Some(start) = s.window_start else { return };
        let Some(window) = self.cfg.scale.as_ref().map(|c| c.window) else { return };
        if s.closed || s.stop.is_some() || now.saturating_duration_since(start) < window {
            return;
        }
        let seconds = now.saturating_duration_since(start).as_secs_f64();
        let evaluations = self.meter.evaluations.load(Ordering::Relaxed);
        let searched = self.meter.searched.load(Ordering::Relaxed);
        let cpu_now = self.cfg.probe.cpu();
        let cpu = match (s.window_cpu, cpu_now) {
            (Some((i0, t0)), Some((i1, t1))) if t1 > t0 => Some(1.0 - (i1.saturating_sub(i0)) as f64 / (t1 - t0) as f64),
            _ => None,
        };
        let w = Window {
            seconds,
            throughput: evaluations.saturating_sub(s.window_evaluations) as f64 / seconds,
            lanes: s.window_area / seconds,
            cpu,
            memory_deferred: s.window_deferred,
        };
        let searched_rate = searched.saturating_sub(s.window_searched) as f64 / seconds;
        let before = s.target;
        let Some(scaler) = s.scaler.as_mut() else { return };
        let d = scaler.step(&w);
        s.target = d.target;
        s.windows.push(json!({
            "Seconds": self.started.elapsed().as_secs_f64(),
            "LanesRunning": w.lanes,
            "LanesBefore": before,
            "LanesAfter": d.target,
            "EvaluationsPerSecond": w.throughput,
            "SearchedPerSecond": searched_rate,
            "Cpu": w.cpu,
            "MemoryDeferred": w.memory_deferred,
            "Limiter": d.limiter.map(Limiter::name),
            "Reason": d.reason,
        }));
        self.say(&format!(
            "autoscale: {:.1} lanes busy, {:.0} evaluations/s, {:.1} searched/s, CPU {}: {} -> {} lanes ({})",
            w.lanes,
            w.throughput,
            searched_rate,
            w.cpu.map_or("unknown".to_string(), |c| format!("{:.0}%", c * 100.0)),
            d.reason,
            d.target,
            match d.change {
                0 => "held".to_string(),
                c if c > 0 => format!("+{c}"),
                c => c.to_string(),
            }
        ));
        s.window_start = Some(now);
        s.window_evaluations = evaluations;
        s.window_searched = searched;
        s.window_cpu = cpu_now;
        s.window_area = 0.0;
        s.window_deferred = false;
    }

    /// Ticks every `Govern::tick` until `finish` (a thread of its own beside the lanes).
    pub fn monitor(&self) {
        while !self.finished.load(Ordering::SeqCst) {
            self.tick(Instant::now());
            std::thread::park_timeout(self.cfg.tick);
        }
    }

    /// Ends the monitor (at its next tick).
    pub fn finish(&self) {
        self.finished.store(true, Ordering::SeqCst);
        self.close();
    }

    /// A short reading for a run's status file (`selfplay-status.json`): the lanes allowed and running, the limiter
    /// that stopped growth, and why the run stopped, if it did.
    pub fn status(&self) -> Value {
        let s = self.lock();
        json!({
            "LanesAllowed": s.target,
            "LanesCeiling": self.lanes,
            "Running": s.running,
            "Limiter": s.scaler.as_ref().and_then(|x| x.limiter).map(Limiter::name),
            "Stopped": s.stop,
        })
    }

    /// The governor's part of the run's report.
    pub fn report(&self) -> Value {
        let s = self.lock();
        let gbs = |b: Option<u64>| b.map(|b| (in_gb(b) * 1000.0).round() / 1000.0);
        json!({
            "Memory": {
                "PeakPrivateGB": gbs((s.peak_private > 0).then_some(s.peak_private)),
                "MinAvailableGB": gbs(s.min_available),
                "MinCommitHeadroomGB": gbs(s.min_headroom.filter(|&h| h != u64::MAX)),
                "PerGameEstimateGB": gbs(Some(s.estimate)),
                "GamesLearntFrom": s.learned,
                "LanesLoaded": s.loaded,
                "LaneModelsGB": gbs(Some(s.lane_cost as u64)),
                "BudgetGB": gbs(self.cfg.budget),
                "ReserveGB": gbs(Some(self.cfg.reserve)),
                "CommitReserveGB": gbs(Some(self.cfg.commit_reserve)),
                "FloorGB": gbs(Some(self.cfg.floor)),
            },
            "LanesRunning": {
                "Ceiling": self.lanes,
                "Min": s.lanes_min,
                "Mean": if s.lanes_seconds > 0.0 { s.lanes_area / s.lanes_seconds } else { 0.0 },
                "Max": s.lanes_max,
            },
            "Deferred": {
                "Admissions": s.deferred,
                "Seconds": s.deferred_seconds,
            },
            "Stopped": s.stop,
            "GamesStopped": s.aborted,
            "Autoscale": self.cfg.scale.as_ref().map(|c| json!({
                "Start": c.start,
                "Step": c.step,
                "WindowSeconds": c.window.as_secs_f64(),
                "MinGain": c.min_gain,
                "CpuCeiling": c.cpu_ceiling,
                "Lanes": s.target,
                "Limiter": s.scaler.as_ref().and_then(|x| x.limiter).map(Limiter::name),
                "Windows": s.windows,
            })),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A probe a test sets.
    struct Fake(Mutex<Memory>);

    impl Probe for Fake {
        fn memory(&self) -> Option<Memory> {
            Some(*self.0.lock().unwrap())
        }
    }

    fn fake(private: f64, available: f64, headroom: f64) -> Arc<Fake> {
        Arc::new(Fake(Mutex::new(Memory { private: gb(private), available: gb(available), commit_headroom: gb(headroom) })))
    }

    fn set(f: &Fake, private: f64, available: f64, headroom: f64) {
        *f.0.lock().unwrap() = Memory { private: gb(private), available: gb(available), commit_headroom: gb(headroom) };
    }

    fn governed(probe: Arc<Fake>, budget: Option<f64>) -> Govern {
        Govern {
            budget: budget.map(gb),
            reserve: gb(6.0),
            commit_reserve: gb(6.0),
            floor: gb(2.0),
            per_game: gb(1.0),
            probe,
            ..Govern::default()
        }
    }

    #[test]
    fn room_names_the_limit_one_more_game_would_pass() {
        let g = governed(fake(0.0, 0.0, 0.0), Some(12.0));
        let m = |p, a, c| Memory { private: gb(p), available: gb(a), commit_headroom: gb(c) };
        assert!(room(&g, m(10.0, 20.0, 20.0), gb(2.0)).is_ok());
        assert!(room(&g, m(10.0, 20.0, 20.0), gb(2.5)).unwrap_err().contains("budget"));
        assert!(room(&g, m(1.0, 7.5, 20.0), gb(2.0)).unwrap_err().contains("available"));
        assert!(room(&g, m(1.0, 20.0, 7.5), gb(2.0)).unwrap_err().contains("commit"));
        assert!(room(&Govern { budget: None, ..g }, m(100.0, 20.0, 20.0), gb(2.0)).is_ok(), "no budget, no process cap");
    }

    #[test]
    fn admission_counts_the_games_in_flight_and_always_lets_one_run() {
        let probe = fake(10.0, 40.0, 40.0);
        let g = Governor::new(governed(probe.clone(), Some(12.5)), 4, None);
        // 10 GB private, a 1 GB guess each: the first with nothing running, the second (10 + 1 pending + 1 = 12), not
        // the third (10 + 2 + 1 = 13 over 12.5).
        let a = g.try_admit(0).unwrap().expect("the first game");
        let b = g.try_admit(1).unwrap().expect("a second fits");
        assert!(g.try_admit(2).unwrap().is_none(), "a third would pass the budget");
        drop((a, b));
        // Memory exhausted beyond every reserve: still one game runs.
        set(&probe, 10.0, 1.0, 1.0);
        let only = g.try_admit(3).unwrap().expect("with none running, one is always let in");
        assert!(g.try_admit(0).unwrap().is_none());
        drop(only);
    }

    #[test]
    fn admission_waits_for_the_reserves_and_for_its_lane() {
        let probe = fake(1.0, 7.5, 40.0);
        let g = Governor::new(governed(probe.clone(), None), 2, None);
        let a = g.try_admit(0).unwrap().unwrap();
        assert!(g.try_admit(0).unwrap().is_none(), "a lane plays one game at a time");
        assert!(g.try_admit(1).unwrap().is_none(), "7.5 GB available less two 1 GB games is under 6");
        set(&probe, 1.0, 40.0, 7.5);
        assert!(g.try_admit(1).unwrap().is_none(), "and so is the commit headroom");
        set(&probe, 1.0, 40.0, 40.0);
        let b = g.try_admit(1).unwrap().expect("room now");
        drop((a, b));
    }

    #[test]
    fn a_waiting_lane_is_admitted_when_a_game_finishes() {
        let probe = fake(10.0, 40.0, 40.0);
        let mut cfg = governed(probe, Some(11.5));
        cfg.tick = Duration::from_millis(5);
        let g = Governor::new(cfg, 2, None);
        let a = g.try_admit(0).unwrap().unwrap();
        let idled = AtomicU32::new(0);
        std::thread::scope(|scope| {
            let waiter = scope.spawn(|| g.admit(1, || {
                idled.fetch_add(1, Ordering::SeqCst);
            }).map(|t| t.lane));
            std::thread::sleep(Duration::from_millis(50));
            a.finish(true);
            assert_eq!(waiter.join().unwrap(), Some(1));
        });
        let r = g.report();
        assert_eq!(r["Deferred"]["Admissions"], 1, "{r}");
        assert_eq!(idled.load(Ordering::SeqCst), 1, "a waiting lane is idled once");
        // A lane let in at once is not idled.
        let b = g.try_admit(0).unwrap();
        drop(b);
        assert!(g.admit(0, || panic!("not idled")).is_some());
    }

    /// A tick that reads the games: one after any start or end is skipped, so each change is followed by two.
    fn read(g: &Governor) {
        g.tick(Instant::now());
        g.tick(Instant::now());
    }

    #[test]
    fn the_estimate_is_learnt_from_the_games_and_falls_as_well_as_rises() {
        let probe = fake(1.0, 40.0, 40.0);
        let g = Governor::new(governed(probe.clone(), None), 2, None);
        let t = g.try_admit(0).unwrap().unwrap();
        set(&probe, 3.0, 40.0, 40.0);
        read(&g);
        t.finish(true);
        assert_eq!(g.estimate(), gb(1.5), "half the way up from the 1 GB guess to a 2 GB game");
        for _ in 0..30 {
            set(&probe, 1.0, 40.0, 40.0);
            let t = g.try_admit(0).unwrap().unwrap();
            set(&probe, 1.5, 40.0, 40.0);
            read(&g);
            t.finish(true);
        }
        assert!(g.estimate() < gb(0.55), "down to games of 0.5 GB: {}", in_gb(g.estimate()));
    }

    #[test]
    fn a_dip_in_the_games_running_does_not_inflate_the_estimate() {
        let probe = fake(1.0, 40.0, 40.0);
        let g = Governor::new(governed(probe.clone(), None), 2, None);
        let a = g.try_admit(0).unwrap().unwrap();
        let b = g.try_admit(1).unwrap().unwrap();
        set(&probe, 3.0, 40.0, 40.0);
        read(&g);
        a.finish(true);
        // The game that ended has not let go of its memory yet: the tick after the change reads no game.
        g.tick(Instant::now());
        set(&probe, 2.0, 40.0, 40.0);
        read(&g);
        b.finish(true);
        assert_eq!(g.estimate(), gb(1.0), "{}", in_gb(g.estimate()));
    }

    #[test]
    fn a_lanes_models_are_not_counted_as_its_games() {
        let probe = fake(1.0, 40.0, 40.0);
        let g = Governor::new(governed(probe.clone(), None), 2, None);
        let t = g.try_admit(0).unwrap().unwrap();
        g.loading(|| set(&probe, 1.5, 40.0, 40.0));
        set(&probe, 2.5, 40.0, 40.0);
        read(&g);
        t.finish(true);
        assert_eq!(g.estimate(), gb(1.0), "1.5 GB grown, 0.5 GB of it the lane's models: {}", in_gb(g.estimate()));
        assert_eq!(g.report()["Memory"]["LaneModelsGB"], 0.5);
    }

    #[test]
    fn the_floor_stops_admission_and_then_the_youngest_game_one_at_a_time() {
        let probe = fake(1.0, 40.0, 40.0);
        let g = Governor::new(governed(probe.clone(), None), 4, None);
        let a = g.try_admit(0).unwrap().unwrap();
        std::thread::sleep(Duration::from_millis(2));
        let b = g.try_admit(1).unwrap().unwrap();
        std::thread::sleep(Duration::from_millis(2));
        let c = g.try_admit(2).unwrap().unwrap();
        // One reading under the floor, or three, is no stop.
        set(&probe, 1.0, 1.5, 40.0);
        for _ in 0..3 {
            g.tick(Instant::now());
        }
        set(&probe, 1.0, 40.0, 40.0);
        g.tick(Instant::now());
        assert!(g.stopped().is_none());
        set(&probe, 1.0, 1.5, 40.0);
        for _ in 0..4 {
            g.tick(Instant::now());
        }
        assert!(g.stopped().unwrap().contains("floor"));
        assert!(g.try_admit(3).is_err(), "no game after a stop");
        assert!(c.abort().load(Ordering::SeqCst), "the youngest is stopped first");
        g.tick(Instant::now());
        assert!(!b.abort().load(Ordering::SeqCst), "not the next while the first is still running");
        drop(c);
        g.tick(Instant::now());
        assert!(b.abort().load(Ordering::SeqCst), "the next once it has let go, while memory stays under");
        // `b` finished as it was being stopped: a game finished, not stopped; and the next is stopped.
        b.finish(true);
        g.tick(Instant::now());
        assert!(a.abort().load(Ordering::SeqCst));
        drop(a);
        assert_eq!(g.report()["GamesStopped"], 2, "c and a");
    }

    #[test]
    fn an_interrupt_lets_the_games_in_flight_finish_and_a_second_stops_them() {
        let probe = fake(1.0, 40.0, 40.0);
        let cfg = governed(probe, None);
        let interrupts = cfg.interrupts.clone();
        let g = Governor::new(cfg, 2, None);
        let a = g.try_admit(0).unwrap().unwrap();
        interrupts.store(1, Ordering::SeqCst);
        g.tick(Instant::now());
        assert_eq!(g.stopped().as_deref(), Some("interrupted"));
        assert!(g.try_admit(1).is_err());
        assert!(!a.abort().load(Ordering::SeqCst), "the game in flight goes on");
        interrupts.store(2, Ordering::SeqCst);
        g.tick(Instant::now());
        assert!(a.abort().load(Ordering::SeqCst));
    }

    fn stop_path(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("cantrip-governor-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir.join("selfplay.stop")
    }

    #[test]
    fn a_stop_file_stops_admission_and_one_saying_now_stops_the_games_in_flight() {
        let path = stop_path("file");
        let cfg = Govern { stop_file: Some(path.clone()), ..governed(fake(1.0, 40.0, 40.0), None) };
        let g = Governor::new(cfg, 2, None);
        let a = g.try_admit(0).unwrap().unwrap();
        g.tick(Instant::now());
        assert!(g.stopped().is_none(), "no file, no stop");
        std::fs::write(&path, "").unwrap();
        g.tick(Instant::now());
        assert!(g.stopped().unwrap().starts_with("stop file"));
        assert!(g.try_admit(1).is_err());
        assert!(!a.abort().load(Ordering::SeqCst), "the game in flight goes on");
        std::fs::write(&path, "now\n").unwrap();
        g.tick(Instant::now());
        assert!(a.abort().load(Ordering::SeqCst), "`now` stops it too");
        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn a_stop_file_there_at_the_start_admits_no_game() {
        let path = stop_path("start");
        std::fs::write(&path, "").unwrap();
        let cfg = Govern { stop_file: Some(path.clone()), ..governed(fake(1.0, 40.0, 40.0), None) };
        let g = Governor::new(cfg, 2, None);
        assert!(g.try_admit(0).is_err());
        assert_eq!(stop_file_level(&path), 1);
        std::fs::remove_file(&path).unwrap();
        assert_eq!(stop_file_level(&path), 0);
        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn a_lease_not_renewed_stops_admission() {
        let path = stop_path("lease").with_file_name("heartbeat");
        std::fs::write(&path, "").unwrap();
        let cfg = Govern { lease: Some((path.clone(), Duration::from_millis(300))), ..governed(fake(1.0, 40.0, 40.0), None) };
        let g = Governor::new(cfg, 2, None);
        let a = g.try_admit(0).unwrap().unwrap();
        g.tick(Instant::now());
        assert!(g.stopped().is_none(), "a fresh lease");
        std::thread::sleep(Duration::from_millis(400));
        g.tick(Instant::now());
        assert!(g.stopped().unwrap().contains("not renewed"));
        assert!(!a.abort().load(Ordering::SeqCst), "the game in flight goes on");
        assert!(lease_expired(&path.with_file_name("none"), Duration::from_secs(60)).unwrap().contains("gone"));
        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }

    fn window(lanes: f64, throughput: f64) -> Window {
        Window { seconds: 45.0, throughput, lanes, cpu: Some(0.5), memory_deferred: false }
    }

    /// Runs the scaler against `throughput(lanes)` for `windows` windows, the lanes running what it allows.
    fn drive(s: &mut Scaler, windows: usize, mut w: impl FnMut(usize) -> Window) -> Vec<Decision> {
        (0..windows).map(|_| s.step(&w(s.target()))).collect()
    }

    #[test]
    fn the_scaler_grows_while_the_gains_continue_and_holds_at_the_plateau() {
        let mut s = Scaler::new(Scale { reprobe: 100, ..Scale::default() }, 96);
        // Linear to 24 lanes, flat after: the GPU's batches full.
        let decisions = drive(&mut s, 40, |l| window(l as f64, 100.0 * l.min(24) as f64));
        assert!((24..=30).contains(&s.target()), "{} lanes: {decisions:#?}", s.target());
        assert_eq!(s.limiter, Some(Limiter::Plateau));
        // Holding, with no oscillation, until the reprobe.
        let held = decisions.iter().rev().take(8).filter(|d| d.change != 0).count();
        assert_eq!(held, 0, "{decisions:#?}");
    }

    #[test]
    fn the_scaler_stops_at_each_limiter() {
        let mut s = Scaler::new(Scale::default(), 10);
        drive(&mut s, 20, |l| window(l as f64, 100.0 * l as f64));
        assert_eq!((s.target(), s.limiter), (10, Some(Limiter::Ceiling)));

        let mut s = Scaler::new(Scale::default(), 96);
        drive(&mut s, 20, |l| Window { cpu: Some(if l >= 12 { 0.97 } else { 0.6 }), ..window(l as f64, 100.0 * l as f64) });
        assert_eq!((s.target(), s.limiter), (12, Some(Limiter::Cpu)));

        let mut s = Scaler::new(Scale::default(), 96);
        drive(&mut s, 20, |l| Window { memory_deferred: l >= 8, ..window(l.min(8) as f64, 100.0 * l.min(8) as f64) });
        assert_eq!((s.target(), s.limiter), (8, Some(Limiter::Memory)));
    }

    #[test]
    fn the_scaler_grows_again_when_a_limiter_clears() {
        let mut s = Scaler::new(Scale::default(), 96);
        let mut busy = true;
        drive(&mut s, 6, |l| Window { cpu: Some(if busy && l >= 8 { 0.99 } else { 0.5 }), ..window(l as f64, 100.0 * l as f64) });
        assert_eq!(s.target(), 8);
        busy = false;
        drive(&mut s, 3, |l| Window { cpu: Some(if busy && l >= 8 { 0.99 } else { 0.5 }), ..window(l as f64, 100.0 * l as f64) });
        assert!(s.target() > 8, "{}", s.target());
    }

    #[test]
    fn the_scaler_backs_off_on_a_drop_and_not_on_one_noisy_window() {
        let mut s = Scaler::new(Scale { reprobe: 100, ..Scale::default() }, 96);
        drive(&mut s, 30, |l| window(l as f64, 100.0 * l.min(16) as f64));
        let held = s.target();
        assert_eq!(s.limiter, Some(Limiter::Plateau));
        // One bad window: nothing.
        let d = s.step(&window(held as f64, 1000.0));
        assert_eq!(d.change, 0, "{d:?}");
        let d = s.step(&window(held as f64, 1600.0));
        assert_eq!(d.change, 0, "{d:?}");
        // Two in a row, 30% down: back off a step.
        s.step(&window(held as f64, 1100.0));
        let d = s.step(&window(held as f64, 1100.0));
        assert_eq!(d.change, -2, "{d:?}");
        assert_eq!(s.limiter, Some(Limiter::Drop));
    }

    #[test]
    fn the_scaler_reprobes_a_plateau_after_its_windows() {
        let mut s = Scaler::new(Scale { reprobe: 3, ..Scale::default() }, 96);
        let mut cap = 12;
        drive(&mut s, 20, |l| window(l as f64, 100.0 * l.min(cap) as f64));
        let first = s.target();
        assert!(first < 20, "{first}");
        // The bottleneck moves (another job freed the GPU): growth resumes at the next reprobe.
        cap = 40;
        drive(&mut s, 30, |l| window(l as f64, 100.0 * l.min(cap) as f64));
        assert!(s.target() >= 36, "{} lanes", s.target());
    }

    #[test]
    fn the_scaler_waits_for_lanes_to_fill() {
        let mut s = Scaler::new(Scale::default(), 96);
        let d = s.step(&window(4.0, 400.0));
        assert_eq!(d.target, 6);
        // Only two of six lanes busy (the run's tail): no growth on that.
        let d = s.step(&window(2.0, 200.0));
        assert_eq!(d.change, 0, "{d:?}");
    }

    #[test]
    fn the_system_probe_reads_this_machine() {
        if cfg!(any(windows, target_os = "linux")) {
            let m = SystemProbe.memory().expect("memory reads");
            assert!(m.private > 0 && m.available > 0 && m.commit_headroom > 0, "{m:?}");
            let (idle, total) = SystemProbe.cpu().expect("CPU times read");
            assert!(total >= idle);
        }
    }
}
