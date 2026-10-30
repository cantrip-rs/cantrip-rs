//! Paying for a spell or ability without a policy step (docs/ai-player-plan.md, "Answers"): mana payment is not a
//! decision the policy learns. `auto_pay` is the scenario runner's payment (`cantrip_engine::pay`), told what a line says
//! (`PayPlan`); `pay` is what the AI player pays with, `auto_pay` first and then the mana abilities a payment needs.

use std::collections::VecDeque;

use cantrip_engine::*;

pub use cantrip_engine::pay::{PayPlan, Spend, auto_pay, pays};

/// The mana types an object's mana ability adds of fixed kinds (`ActivatedDef::adds`): what an option the engine offers
/// with no `mana` of its own (a basic land's, each of Plateau's two) yields. Empty where the ability is not found.
fn fixed_yield(game: &Game, option: &ActionOption) -> Vec<Mana> {
    let Some(source) = option.source else {
        return Vec::new();
    };
    if option.index >= PLAYER_MANA_ABILITY {
        return Vec::new();
    }
    let Some(ability) = game.chars(source).abilities().nth(option.index as usize) else {
        return Vec::new();
    };
    let Some(def) = game.activated_ability(ability) else {
        return Vec::new();
    };
    Mana::ALL
        .iter()
        .copied()
        .filter(|&m| def.adds.0[m as usize] > 0)
        .collect()
}

/// The mana a mana ability option adds that pays something of `owed`, as the action that adds it; `None` if it adds
/// nothing owed. A choice of color takes the first color owed; an option that names no mana is judged by what its
/// ability adds (`fixed_yield`).
fn mana_action(game: &Game, option: &ActionOption, owed: &ManaCost) -> Option<Action> {
    if option.kind != ActionKind::ActivateAbility
        || option.targets_required > 0 && !option.targets_optional
    {
        return None;
    }
    let mut action = Action::of(option);
    if !option.color_choices.is_empty() {
        let colors = [
            Color::White,
            Color::Blue,
            Color::Black,
            Color::Red,
            Color::Green,
        ];
        let owed_color = colors
            .iter()
            .copied()
            .filter(|&c| option.color_choices.has(c))
            .find(|&c| pays(owed, Mana::ALL[c as usize]));
        let color = owed_color.or_else(|| {
            colors
                .iter()
                .copied()
                .find(|&c| option.color_choices.has(c))
        })?;
        action.color = Some(color);
        if option.mana == Some(Mana::C) {
            action.mana = None;
        }
        return Some(action);
    }
    match option.mana {
        Some(m) if pays(owed, m) => Some(action),
        Some(_) => None,
        None => fixed_yield(game, option)
            .into_iter()
            .any(|m| pays(owed, m))
            .then_some(action),
    }
}

/// The kinds of mana a mana ability option could add: its colors to choose, the mana it names, what its ability adds.
fn kinds(game: &Game, option: &ActionOption) -> Vec<Mana> {
    let colors = [
        Color::White,
        Color::Blue,
        Color::Black,
        Color::Red,
        Color::Green,
    ];
    let mut kinds: Vec<Mana> = colors
        .iter()
        .filter(|&&c| option.color_choices.has(c))
        .map(|&c| Mana::ALL[c as usize])
        .collect();
    kinds.extend(option.mana);
    kinds.extend(fixed_yield(game, option));
    kinds.sort_by_key(|&m| m as usize);
    kinds.dedup();
    kinds
}

/// Each source in `sources` (the kinds of mana it could add) given to one of `symbols` (the kinds each could be paid
/// with), no source twice: the sources, in the order of the symbols, with the mana each pays. `None` if there is none.
fn cover(
    symbols: &[Vec<Mana>],
    sources: &[Vec<Mana>],
    used: &mut Vec<bool>,
) -> Option<Vec<(usize, Mana)>> {
    let Some((first, rest)) = symbols.split_first() else {
        return Some(Vec::new());
    };
    for (i, kinds) in sources.iter().enumerate() {
        if used[i] {
            continue;
        }
        let Some(&m) = first.iter().find(|m| kinds.contains(m)) else {
            continue;
        };
        used[i] = true;
        if let Some(mut more) = cover(rest, sources, used) {
            more.insert(0, (i, m));
            return Some(more);
        }
        used[i] = false;
    }
    None
}

/// What the search wants to do next with the mana left: the mana cost of each of the
/// seat's follow-up plays, with how much the search wants it (its visits' share, or its prior). The autotapper leaves open
/// what pays the most of it (`tap`). Set for a seat's payments by whoever knows the plan (`with_follow_ups`): the search
/// for a line's payments (the options at the node the cast was chosen at), the AI host for the real game's.
pub type FollowUps = Vec<(ManaCost, f32)>;

thread_local! {
    static FOLLOW_UPS: std::cell::RefCell<Option<(PlayerId, FollowUps)>> = const { std::cell::RefCell::new(None) };
}

thread_local! {
    static KEEP: std::cell::RefCell<Option<(PlayerId, Vec<ObjectId>)>> = const { std::cell::RefCell::new(None) };
}

/// Runs `run` with the sources `keep` to be left untapped by `player`'s payments where a way to pay does (`tap`), as
/// they were after: the class a search chose for a cast it split (`classes`).
pub fn with_payment_keep<T>(player: PlayerId, keep: &[ObjectId], run: impl FnOnce() -> T) -> T {
    let before = KEEP.with(|k| std::mem::replace(&mut *k.borrow_mut(), Some((player, keep.to_vec()))));
    let out = run();
    KEEP.with(|k| *k.borrow_mut() = before);
    out
}

/// Runs `run` with `follow` as the follow-ups `player`'s payments keep payable (`FollowUps`; another player's payments in
/// it keep none), as they were after.
pub fn with_follow_ups<T>(player: PlayerId, follow: &[(ManaCost, f32)], run: impl FnOnce() -> T) -> T {
    let before = FOLLOW_UPS.with(|f| std::mem::replace(&mut *f.borrow_mut(), Some((player, follow.to_vec()))));
    let out = run();
    FOLLOW_UPS.with(|f| *f.borrow_mut() = before);
    out
}

/// The mana a priority option would cost to take, for a follow-up (`FollowUps`): a spell's mana cost as printed, an
/// ability's mana cost; `None` for one that costs no mana (a land, a mana-free ability). The engine says what it costs
/// when it is taken; this is only what the autotapper keeps open for.
pub fn option_cost(game: &Game, option: &ActionOption) -> Option<ManaCost> {
    let source = option.source?;
    let cost = match option.kind {
        ActionKind::CastSpell => game.cards.get(game.state.obj(source).card).cost,
        ActionKind::ActivateAbility if option.index < PLAYER_MANA_ABILITY => {
            let ability = game.chars(source).abilities().nth(option.index as usize)?;
            let def = game.activated_ability(ability)?;
            if def.is_mana_ability() {
                return None;
            }
            def.cost.mana
        }
        _ => return None,
    };
    (!cost.is_zero()).then_some(cost)
}

/// How much the autotapper would rather keep a mana source than tap it: lower is tapped first. Modelled on MTG Arena's
/// autotapper, in order of what matters:
///
/// 1. A last resort: a source that would stay tapped through the next untap step (`Game::stays_tapped`), or whose ability
///    costs or does more than tapping (life, sacrifice, counters, energy, another permanent, a discard, mana - a filter
///    land; or words that hurt, "deals ... damage to you").
/// 2. A source with an ability that is not a mana ability (a land that becomes a creature, a utility land, an artifact
///    with another use) after a plain one: tapping it for mana spends that use too.
/// 3. A creature (not a land) after a land or another permanent: it may yet attack or block.
/// 4. A source that adds fewer kinds of mana before one that adds more (a basic before a dual land).
///
/// Which sources a payment leaves untapped is chosen before this ordering, by what the plan wants to pay for next (`tap`);
/// the ordering breaks ties within that choice, and with no plan to go on the colors the hand needs come last.
fn keep_score(game: &Game, option: &ActionOption, kinds: &[Mana]) -> (u8, u8, u8, usize) {
    let source = option.source;
    let def = source.and_then(|s| {
        game.chars(s)
            .abilities()
            .nth(option.index as usize)
            .and_then(|a| game.activated_ability(a))
    });
    let costly = def.is_some_and(|d| {
        let c = &d.cost;
        c.life > 0
            || c.sacrifice_self
            || c.remove_counters.is_some()
            || c.energy > 0
            || c.permanents.is_some()
            || c.discard_self
            || !c.mana.is_zero()
            || d.text.contains("damage to you")
            || d.text.contains("Exert")
    }) || source.is_some_and(|s| game.stays_tapped(s));
    let utility = source.is_some_and(|s| {
        game.chars(s)
            .abilities()
            .filter_map(|a| game.activated_ability(a))
            .any(|d| !d.is_mana_ability())
    });
    let creature = source.is_some_and(|s| {
        let t = game.chars(s).types;
        t.has(CardType::Creature) && !t.has(CardType::Land)
    });
    (u8::from(costly), u8::from(utility), u8::from(creature), kinds.len())
}

/// The colored symbols of the cards in `player`'s hand, by mana type: what the colors it keeps open are for, where there
/// is no plan to say (`FollowUps` empty).
fn hand_needs(game: &Game, player: PlayerId) -> [u32; 6] {
    let mut need = [0u32; 6];
    for &id in game.state.zone(Zone::hand(player)) {
        let cost = game.cards.get(game.state.obj(id).card).cost;
        for (m, n) in cost.symbols.iter().enumerate() {
            need[m] += u32::from(*n);
        }
    }
    need
}

/// How much mana one activation of the option adds: its ability's stated yield, else one.
fn amount(game: &Game, option: &ActionOption) -> u32 {
    let Some(source) = option.source else { return 1 };
    if option.index >= PLAYER_MANA_ABILITY {
        return 1;
    }
    game.chars(source)
        .abilities()
        .nth(option.index as usize)
        .and_then(|a| game.activated_ability(a))
        .map(|d| d.adds.0.iter().map(|&n| u32::from(n)).sum::<u32>())
        .filter(|&n| n > 0)
        .unwrap_or(1)
}

/// One permanent (or player ability) a payment can tap: its options (indices into the request's), the kinds of mana it
/// can add, how much a tap adds, and its `keep_score`.
#[derive(Clone)]
struct Source {
    options: Vec<usize>,
    kinds: Vec<Mana>,
    amount: u32,
    key: (u8, u8, u8, usize),
    id: Option<ObjectId>,
}

/// The symbols a cost owes, each with the kinds of mana that pay it - colored, hybrid, two-or-generic ({2/W}: its color,
/// preferred to two generic) and Phyrexian (its color: life is the engine's other answer) - and how much generic.
fn owed_symbols(owed: &ManaCost) -> (Vec<Vec<Mana>>, u32) {
    let mut symbols: Vec<Vec<Mana>> = Vec::new();
    for m in Mana::ALL {
        for _ in 0..owed.symbols[m as usize] {
            symbols.push(vec![m]);
        }
    }
    symbols.extend(owed.hybrid.iter().flatten().map(|&(a, b)| vec![a, b]));
    let colors = [Mana::W, Mana::U, Mana::B, Mana::R, Mana::G];
    for (i, &n) in owed.twobrid.iter().enumerate().chain(owed.phyrexian.iter().enumerate()) {
        for _ in 0..n {
            symbols.push(vec![colors[i]]);
        }
    }
    (symbols, u32::from(owed.generic))
}

/// Whether the sources `used` (indices into `sources`) pay the symbols and the generic part, and the mana their taps add
/// that it does not use: each symbol matched to a source of its kind (a source of `amount` mana is that many), and what is
/// left of them paying the generic part. `None` if they do not pay it.
fn pays_with(symbols: &[Vec<Mana>], generic: u32, sources: &[Source], used: &[usize]) -> Option<u32> {
    let slots: Vec<Vec<Mana>> =
        used.iter().flat_map(|&i| std::iter::repeat_n(sources[i].kinds.clone(), sources[i].amount as usize)).collect();
    let mut taken = vec![false; slots.len()];
    cover(symbols, &slots, &mut taken)?;
    let left = taken.iter().filter(|t| !**t).count() as u32;
    (left >= generic).then_some(left - generic)
}

/// Whether the sources `left` could pay `cost`.
fn could_pay(cost: &ManaCost, sources: &[Source], left: &[usize]) -> bool {
    let (symbols, generic) = owed_symbols(cost);
    pays_with(&symbols, generic, sources, left).is_some()
}

/// The ways to pay from `sources` with none to spare: every set of them that pays and has no smaller such set inside it,
/// smallest first, at most `CAP`; each with the mana it wastes.
fn payments(symbols: &[Vec<Mana>], generic: u32, sources: &[Source]) -> Vec<(Vec<usize>, u32)> {
    const CAP: usize = 64;
    #[allow(clippy::too_many_arguments)]
    fn grow(
        at: usize,
        chosen: &mut Vec<usize>,
        size: usize,
        capacity: u32,
        need: u32,
        sources: &[Source],
        symbols: &[Vec<Mana>],
        generic: u32,
        out: &mut Vec<(Vec<usize>, u32)>,
    ) {
        if out.len() >= CAP {
            return;
        }
        if chosen.len() == size {
            if capacity >= need
                && !out.iter().any(|(o, _)| o.iter().all(|i| chosen.contains(i)))
                && let Some(waste) = pays_with(symbols, generic, sources, chosen)
            {
                out.push((chosen.clone(), waste));
            }
            return;
        }
        for i in at..sources.len() {
            chosen.push(i);
            grow(i + 1, chosen, size, capacity + sources[i].amount, need, sources, symbols, generic, out);
            chosen.pop();
        }
    }
    let need = symbols.len() as u32 + generic;
    let mut out: Vec<(Vec<usize>, u32)> = Vec::new();
    // Nothing to look for if all of them together do not pay (a payment `tap` answers otherwise).
    let all: Vec<usize> = (0..sources.len()).collect();
    if pays_with(symbols, generic, sources, &all).is_none() {
        return out;
    }
    for size in 1..=sources.len() {
        let mut chosen = Vec::new();
        grow(0, &mut chosen, size, 0, need, sources, symbols, generic, &mut out);
        if out.len() >= CAP {
            break;
        }
    }
    out
}

/// A way to pay all that is owed (`payments`), as `ways` judges it: the sources it taps, the weight of the follow-ups the
/// sources it leaves untapped could pay (`FollowUps`), the mana it wastes, its sources' `keep_score`s worst first, and
/// what it leaves untapped (the objects, and their kinds and amounts: its class).
struct Way {
    used: Vec<usize>,
    kept: f32,
    /// Which follow-ups it keeps payable, a bit each (the first 64).
    kept_mask: u64,
    waste: u32,
    keys: Vec<(u8, u8, u8, usize)>,
    left: Vec<ObjectId>,
    class: Vec<(Vec<Mana>, u32)>,
}

/// The sources a payment's options give (a permanent's options together, as one tap of it is all it gives) in
/// `keep_score` order, and every way to pay `owed` with them judged against `follow`. No ways with more than 16 sources
/// (the ordering alone then).
fn ways(game: &Game, options: &[ActionOption], owed: &ManaCost, follow: &[(ManaCost, f32)]) -> (Vec<Source>, Vec<Way>) {
    let player = options.iter().find_map(|o| o.source).map(|s| game.state.obj(s).controller);
    let mut candidates: Vec<(usize, Vec<Mana>, (u8, u8, u8, usize))> = options
        .iter()
        .enumerate()
        .filter(|(_, o)| mana_action(game, o, owed).is_some())
        .map(|(i, o)| {
            let k = kinds(game, o);
            let key = keep_score(game, o, &k);
            (i, k, key)
        })
        .collect();
    // With no plan, the hand's colors are what is kept open last (the old autotapper's last rule).
    let need = if follow.is_empty() { player.map_or([0; 6], |p| hand_needs(game, p)) } else { [0; 6] };
    candidates.sort_by_key(|(_, k, key)| (*key, k.iter().map(|&m| need[m as usize]).sum::<u32>()));
    let mut sources: Vec<Source> = Vec::new();
    let mut of: Vec<(ObjectId, usize)> = Vec::new();
    for (i, k, key) in &candidates {
        let o = &options[*i];
        match o.source.and_then(|s| of.iter().find(|(id, _)| *id == s).map(|(_, at)| *at)) {
            Some(at) => {
                let src = &mut sources[at];
                src.options.push(*i);
                for &m in k {
                    if !src.kinds.contains(&m) {
                        src.kinds.push(m);
                    }
                }
                src.amount = src.amount.max(amount(game, o));
            }
            None => {
                if let Some(s) = o.source {
                    of.push((s, sources.len()));
                }
                sources.push(Source { options: vec![*i], kinds: k.clone(), amount: amount(game, o), key: *key, id: o.source });
            }
        }
    }
    let (symbols, generic) = owed_symbols(owed);
    if sources.len() > 16 {
        return (sources, Vec::new());
    }
    let ways = payments(&symbols, generic, &sources)
        .into_iter()
        .map(|(used, waste)| {
            let left: Vec<usize> = (0..sources.len()).filter(|i| !used.contains(i)).collect();
            let (mut kept, mut kept_mask) = (0.0f32, 0u64);
            for (j, (cost, w)) in follow.iter().enumerate() {
                if could_pay(cost, &sources, &left) {
                    kept += w;
                    if j < 64 {
                        kept_mask |= 1 << j;
                    }
                }
            }
            let mut keys: Vec<(u8, u8, u8, usize)> = used.iter().map(|&i| sources[i].key).collect();
            keys.sort_unstable_by(|a, b| b.cmp(a));
            let mut class: Vec<(Vec<Mana>, u32)> = left
                .iter()
                .map(|&i| {
                    let mut k = sources[i].kinds.clone();
                    k.sort_by_key(|&m| m as usize);
                    (k, sources[i].amount)
                })
                .collect();
            class.sort_by(|a, b| (a.0.iter().map(|&m| m as usize).collect::<Vec<_>>(), a.1).cmp(&(b.0.iter().map(|&m| m as usize).collect(), b.1)));
            let left = left.iter().filter_map(|&i| sources[i].id).collect();
            Way { used, kept, kept_mask, waste, keys, left, class }
        })
        .collect();
    (sources, ways)
}

/// The better of two ways: more of the plan kept payable, then less mana wasted, then the sources the Arena-like rules
/// would rather tap (`keep_score`, compared worst first).
fn better(a: &Way, b: &Way) -> std::cmp::Ordering {
    a.kept
        .partial_cmp(&b.kept)
        .unwrap_or(std::cmp::Ordering::Equal)
        .then(b.waste.cmp(&a.waste))
        .then(b.keys.cmp(&a.keys))
}

/// The classes of the ways to pay `cost` from the mana abilities among `options` (a priority's, or a payment's): ways that
/// leave sources of the same kinds and amounts untapped are one class (`Way`). Each class's best way, judged against
/// `follow`, as the objects it leaves untapped, the follow-ups' weight they keep payable and which of them (a bit each),
/// best class first: what the
/// search splits a cast by where the plan does not settle it (`search::Config::pay_split`).
pub fn classes(game: &Game, options: &[ActionOption], cost: &ManaCost, follow: &[(ManaCost, f32)]) -> Vec<(Vec<ObjectId>, f32, u64)> {
    let mana: Vec<ActionOption> = options.iter().filter(|o| crate::atomic::is_mana_ability(game, o)).cloned().collect();
    let (_, ways) = ways(game, &mana, cost, follow);
    let mut best: Vec<&Way> = Vec::new();
    for w in &ways {
        match best.iter_mut().find(|b| b.class == w.class) {
            Some(b) => {
                if better(w, b).is_gt() {
                    *b = w;
                }
            }
            None => best.push(w),
        }
    }
    best.sort_by(|a, b| better(b, a));
    best.into_iter().map(|w| (w.left.clone(), w.kept, w.kept_mask)).collect()
}

/// The mana ability to activate next towards `owed`.
///
/// The ways to pay all that is owed are found first (`payments`) and grouped by what each leaves untapped: two ways that
/// leave sources of the same kinds and amounts open are one class (`Way`). The way taken keeps open, first, the sources the
/// search chose to keep for this payment where it split the cast by class (`with_payment_keep`); then the most of what the
/// plan wants to do next (`FollowUps`), so the best class wins; within it, and between classes the plan does not tell
/// apart, the way that wastes the least mana (a source of two spent on one generic), then the one tapping what the
/// Arena-like rules would rather tap (`keep_score`, compared worst first). One source of that way is tapped, for the first
/// symbol it pays (colored ones before the generic part): so the generic part is never paid with the only source of a
/// color still owed, and a dual land is not spent on a color a basic land adds. With no way found (more than 16 sources,
/// or a cost the matching can't see), the first ability in `keep_score` order that adds anything owed.
fn tap(game: &Game, options: &[ActionOption], owed: &ManaCost) -> Option<Action> {
    if CLASSIC.with(|c| c.get()) {
        return tap_classic(game, options, owed);
    }
    let player = options.iter().find_map(|o| o.source).map(|s| game.state.obj(s).controller);
    let follow: FollowUps = FOLLOW_UPS.with(|f| match &*f.borrow() {
        Some((p, follow)) if Some(*p) == player => follow.clone(),
        _ => Vec::new(),
    });
    let keep: Vec<ObjectId> = KEEP.with(|k| match &*k.borrow() {
        Some((p, keep)) if Some(*p) == player => keep.clone(),
        _ => Vec::new(),
    });
    let (sources, ways) = ways(game, options, owed, &follow);
    let keeps = |w: &Way| keep.iter().all(|id| w.left.contains(id));
    let chosen = ways.iter().max_by(|a, b| keeps(a).cmp(&keeps(b)).then_with(|| better(a, b)));
    if let Some(way) = chosen {
        let (symbols, _) = owed_symbols(owed);
        let used = &way.used;
        let slots: Vec<Vec<Mana>> = used.iter().map(|&i| sources[i].kinds.clone()).collect();
        let (src, m) = match first_paid(&symbols, &slots) {
            Some((j, m)) => (used[j], Some(m)),
            None => (used[0], None),
        };
        let action = match m {
            // The option of that source that adds the mana wanted.
            Some(m) => sources[src]
                .options
                .iter()
                .map(|&i| &options[i])
                .find(|o| kinds(game, o).contains(&m))
                .map(|o| action_for(o, m)),
            None => sources[src].options.iter().find_map(|&i| mana_action(game, &options[i], owed)),
        };
        if action.is_some() {
            return action;
        }
    }
    sources.iter().flat_map(|s| s.options.iter()).find_map(|&i| mana_action(game, &options[i], owed))
}

/// The source of a way (an index into `slots`) and the mana it pays the first symbol with, the symbols matched to the
/// sources as `cover` matches them; with sources of more than one mana, the first symbol any of them pays. `None` with no
/// symbol owed (the generic part only).
fn first_paid(symbols: &[Vec<Mana>], slots: &[Vec<Mana>]) -> Option<(usize, Mana)> {
    if symbols.is_empty() {
        return None;
    }
    let mut used = vec![false; slots.len()];
    if let Some(plan) = cover(symbols, slots, &mut used) {
        return plan.first().copied();
    }
    symbols.iter().find_map(|kinds| {
        slots.iter().enumerate().find_map(|(i, k)| kinds.iter().find(|m| k.contains(m)).map(|&m| (i, m)))
    })
}

/// The action that activates `option` for mana `m`: its color chosen where it chooses one.
fn action_for(option: &ActionOption, m: Mana) -> Action {
    let mut action = Action::of(option);
    if !option.color_choices.is_empty() {
        if m == Mana::C {
            action.color = None;
        } else {
            action.color = Some([Color::White, Color::Blue, Color::Black, Color::Red, Color::Green][m as usize]);
            if option.mana == Some(Mana::C) {
                action.mana = None;
            }
        }
    }
    action
}

/// The autotapper before it was told the plan (`with_classic`): the Arena-like order alone, the hand's colors last,
/// kept to measure against: a new autotapper is judged against the old at equal settings.
fn keep_score_classic(game: &Game, option: &ActionOption, kinds: &[Mana], need: &[u32; 6]) -> (u8, u8, usize, u32) {
    let source = option.source;
    let def = source.and_then(|s| {
        game.chars(s)
            .abilities()
            .nth(option.index as usize)
            .and_then(|a| game.activated_ability(a))
    });
    let costly = def.is_some_and(|d| {
        let c = &d.cost;
        c.life > 0
            || c.sacrifice_self
            || c.remove_counters.is_some()
            || c.energy > 0
            || c.permanents.is_some()
            || c.discard_self
            || !c.mana.is_zero()
            || d.text.contains("damage to you")
            || d.text.contains("Exert")
    });
    let creature = source.is_some_and(|s| {
        let t = game.chars(s).types;
        t.has(CardType::Creature) && !t.has(CardType::Land)
    });
    let needed: u32 = kinds.iter().map(|&m| need[m as usize]).sum();
    (u8::from(costly), u8::from(creature), kinds.len(), needed)
}

/// `tap` before it was told the plan: the colored symbols matched to sources in `keep_score_classic` order, the first of
/// them tapped; with only generic mana owed, the first source in that order.
fn tap_classic(game: &Game, options: &[ActionOption], owed: &ManaCost) -> Option<Action> {
    let player = options.iter().find_map(|o| o.source).map(|s| game.state.obj(s).controller);
    let need = player.map_or([0; 6], |p| hand_needs(game, p));
    let mut candidates: Vec<(&ActionOption, Vec<Mana>)> = options
        .iter()
        .filter(|o| mana_action(game, o, owed).is_some())
        .map(|o| (o, kinds(game, o)))
        .collect();
    candidates.sort_by_key(|(o, k)| keep_score_classic(game, o, k, &need));
    let mut symbols: Vec<Vec<Mana>> = Vec::new();
    for m in Mana::ALL {
        for _ in 0..owed.symbols[m as usize] {
            symbols.push(vec![m]);
        }
    }
    symbols.extend(owed.hybrid.iter().flatten().map(|&(a, b)| vec![a, b]));
    let sources: Vec<Vec<Mana>> = candidates.iter().map(|(_, k)| k.clone()).collect();
    if !symbols.is_empty()
        && let Some(plan) = cover(&symbols, &sources, &mut vec![false; sources.len()])
        && let Some(&(i, m)) = plan.first()
    {
        let option = candidates[i].0;
        let mut action = Action::of(option);
        if !option.color_choices.is_empty() {
            if m == Mana::C {
                action.color = None;
            } else {
                action.color = Some(
                    [
                        Color::White,
                        Color::Blue,
                        Color::Black,
                        Color::Red,
                        Color::Green,
                    ][m as usize],
                );
                if option.mana == Some(Mana::C) {
                    action.mana = None;
                }
            }
        }
        return Some(action);
    }
    candidates.iter().find_map(|(o, _)| mana_action(game, o, owed))
}

thread_local! {
    static CLASSIC: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// Runs `run` with the old autotapper (`tap_classic`) where `classic`, as it was after: for measuring the new one.
pub fn with_classic<T>(classic: bool, run: impl FnOnce() -> T) -> T {
    let before = CLASSIC.with(|c| c.replace(classic));
    let out = run();
    CLASSIC.with(|c| c.set(before));
    out
}

/// What the AI player pays a payment with: for `PayCosts` (announcing `x` where it is being announced), `auto_pay`,
/// then a mana ability that adds mana still owed; for `UnlessPays`, X raised or lowered to `x`, then paying once the
/// pool covers it, a mana ability while it does not. `None` where it cannot pay: the caller cancels or declines.
pub fn pay(game: &Game, request: &Request, x: Option<u16>) -> Option<Answer> {
    match &request.kind {
        RequestKind::PayCosts {
            remaining, options, ..
        } => {
            let mut plan = PayPlan::new(x, VecDeque::new());
            if let Ok(Some(answer)) = auto_pay(game, request, &mut plan) {
                return Some(answer);
            }
            tap(game, options, remaining).map(Answer::Act)
        }
        RequestKind::UnlessPays {
            remaining,
            options,
            x: now,
            ..
        } => {
            let offers = |kind: ActionKind| {
                options
                    .iter()
                    .find(|o| o.kind == kind)
                    .map(|o| Answer::Act(Action::of(o)))
            };
            if let (Some(now), Some(want)) = (*now, x) {
                if now < want
                    && let Some(a) = offers(ActionKind::IncreaseX)
                {
                    return Some(a);
                }
                if now > want
                    && let Some(a) = offers(ActionKind::DecreaseX)
                {
                    return Some(a);
                }
            }
            if remaining.is_zero() {
                if let Some(a) = offers(ActionKind::PayCost) {
                    return Some(a);
                }
            }
            tap(game, options, remaining)
                .map(Answer::Act)
                .or_else(|| offers(ActionKind::PayCost))
        }
        _ => None,
    }
}
