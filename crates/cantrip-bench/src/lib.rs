//! A seat that answers at random, as `src/Cantrip.Client/RandomPlay.cs` does: at priority, any affordable action
//! that is not a mana ability, or a pass, with even odds among them; costs paid automatically; every hand kept; every
//! other question answered with a random legal answer.

use cantrip_engine::*;

/// The seat's own generator, apart from the game's.
pub struct Random(u64);

impl Random {
    pub fn new(seed: u64) -> Random {
        Random(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1)
    }

    pub fn below(&mut self, n: usize) -> usize {
        // xorshift64*
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        let r = self.0.wrapping_mul(0x2545_F491_4F6C_DD1D);
        if n == 0 { 0 } else { (r % n as u64) as usize }
    }

    pub fn coin(&mut self) -> bool {
        self.below(2) == 0
    }

    fn some<T: Clone>(&mut self, items: &[T], count: usize) -> Vec<T> {
        let mut pool = items.to_vec();
        let mut out = Vec::new();
        for _ in 0..count.min(pool.len()) {
            out.push(pool.swap_remove(self.below(pool.len())));
        }
        out
    }
}

fn mana_ability(game: &Game, option: &ActionOption) -> Option<&'static ActivatedDef> {
    let source = option.source?;
    let ability = game.chars(source).abilities().nth(option.index as usize)?;
    match game.cards.ability(ability) {
        AbilityDef::Activated(a) if a.is_mana_ability() => Some(a),
        _ => None,
    }
}

/// The mana cost of what an option would do.
fn cost_of(game: &Game, option: &ActionOption) -> Option<ManaCost> {
    let source = option.source?;
    match option.kind {
        ActionKind::CastSpell => {
            let def = game.cards.get(game.state.obj(source).card);
            // The card's own alternative costs come first; the special ways (a back face, a split half, an Adventure
            // or Omen) cast a face at its own cost.
            match (option.index, def.split) {
                (0, _) => Some(def.cost),
                (WAY_BACK_FACE, _) => def.back.map(|b| b.cost),
                (WAY_SPLIT_LEFT, Some((left, _))) => Some(left.cost),
                (WAY_SPLIT_RIGHT, Some((_, right))) => Some(right.cost),
                (way, _) => def.ways.get(way as usize - 1).map(|w| w.cost.mana).or_else(|| def.inset_face(way).map(|f| f.cost)),
            }
        }
        ActionKind::ActivateAbility => {
            let ability = game.chars(source).abilities().nth(option.index as usize)?;
            match game.cards.ability(ability) {
                AbilityDef::Activated(a) => Some(a.cost.mana),
                _ => None,
            }
        }
        _ => Some(ManaCost::ZERO),
    }
}

/// Whether the pool and the mana abilities on offer could pay `cost`.
fn affordable(game: &Game, player: PlayerId, options: &[ActionOption], cost: &ManaCost) -> bool {
    let mut have = game.state.player(player).pool.0;
    let mut any = 0u32;
    for option in options {
        if let Some(a) = mana_ability(game, option) {
            for (i, n) in a.adds.0.iter().enumerate() {
                have[i] += n;
            }
            any += a.adds_any as u32;
        }
    }
    for mana in Mana::ALL {
        let need = u32::from(cost.symbols[mana as usize]);
        let take = need.min(have[mana as usize]);
        have[mana as usize] -= take;
        let short = need - take;
        if short > any {
            return false;
        }
        any -= short;
    }
    for (a, b) in cost.hybrid.iter().flatten() {
        if have[*a as usize] > 0 {
            have[*a as usize] -= 1;
        } else if have[*b as usize] > 0 {
            have[*b as usize] -= 1;
        } else if any > 0 {
            any -= 1;
        } else {
            return false;
        }
    }
    have.iter().map(|&n| n as u32).sum::<u32>() + any >= cost.generic as u32
}

fn fill(random: &mut Random, option: &ActionOption) -> Action {
    let mut action = Action::of(option);
    if option.targets_required > 0 && !option.target_candidates.is_empty() {
        let optional_none = option.targets_optional && random.below(option.target_candidates.len() + 1) == 0;
        if !optional_none {
            action.targets = random.some(&option.target_candidates, option.targets_required as usize);
        }
    }
    if !option.color_choices.is_empty() {
        let colors: Vec<Color> = option.color_choices.iter().collect();
        action.color = Some(colors[random.below(colors.len())]);
    }
    action
}

fn pays(owed: &ManaCost, mana: Mana) -> bool {
    owed.generic > 0 || owed.symbols[mana as usize] > 0 || owed.hybrid.iter().flatten().any(|(a, b)| *a == mana || *b == mana)
}

/// Pays as the C# AutoPlay does: pool mana that pays something, then a mana ability that makes something owed,
/// else gives up.
fn pay(game: &Game, random: &mut Random, remaining: &ManaCost, options: &[ActionOption]) -> Answer {
    if let Some(o) = options.iter().find(|o| o.kind == ActionKind::AssignMana && o.mana.is_some_and(|m| pays(remaining, m))) {
        return Answer::Act(Action::of(o));
    }
    for o in options.iter().filter(|o| o.kind == ActionKind::ActivateAbility) {
        if let Some(a) = mana_ability(game, o)
            && (a.adds_any > 0 || Mana::ALL.iter().any(|&m| a.adds.0[m as usize] > 0 && pays(remaining, m)))
        {
            return Answer::Act(fill(random, o));
        }
    }
    match options.iter().find(|o| o.kind == ActionKind::CancelCasting) {
        Some(o) => Answer::Act(Action::of(o)),
        None => Answer::Cancel,
    }
}

pub fn decide(game: &Game, random: &mut Random, request: &Request) -> Answer {
    match &request.kind {
        RequestKind::Priority { options } => {
            let takeable: Vec<&ActionOption> = options
                .iter()
                .filter(|o| match o.kind {
                    ActionKind::PlayLand => true,
                    ActionKind::CastSpell | ActionKind::ActivateAbility => {
                        mana_ability(game, o).is_none()
                            && cost_of(game, o).is_some_and(|c| affordable(game, request.player, options, &c))
                    }
                    _ => false,
                })
                .collect();
            let draw = random.below(takeable.len() + 1);
            if draw == takeable.len() { Answer::Pass } else { Answer::Act(fill(random, takeable[draw])) }
        }
        RequestKind::PayCosts { announcing: true, options, .. } => {
            // Announces X or kicks as nothing: the seat keeps to what it judged affordable.
            Answer::Act(Action::of(options.iter().find(|o| o.kind == ActionKind::ConfirmCasting).expect("confirm")))
        }
        RequestKind::PayCosts { remaining, options, .. } => pay(game, random, remaining, options),
        RequestKind::UnlessPays { options, .. } => {
            if let Some(o) = options.iter().find(|o| o.kind == ActionKind::PayCost)
                && random.coin()
            {
                return Answer::Act(Action::of(o));
            }
            let mana: Vec<&ActionOption> = options.iter().filter(|o| o.kind == ActionKind::ActivateAbility).collect();
            if !mana.is_empty() && random.coin() {
                let pick = random.below(mana.len());
                return Answer::Act(fill(random, mana[pick]));
            }
            Answer::Act(Action::of(options.iter().find(|o| o.kind == ActionKind::DeclineCost).expect("decline")))
        }
        RequestKind::ChooseCards { rule, .. } => {
            let eligible = first_choices(game, request);
            let (count, up_to) = match rule.count {
                Count::Exactly(n) => (n as usize, false),
                Count::UpTo(n) => (n as usize, true),
                Count::AnyNumber => (eligible.len(), true),
                Count::AsManyAsCan(n) => ((n as usize).min(eligible.len()), false),
            };
            let n = if up_to { random.below(count + 1) } else { count };
            Answer::Cards(fitting(game, request, random.some(&eligible, n)))
        }
        RequestKind::ChooseTarget { source, candidates, least, count, together, .. } => {
            let n = *least as usize + random.below((*count - *least) as usize + 1);
            let chosen = random.some(candidates, n);
            // A slot whose targets together are judged ("with total mana value X or less"): a random choice is cut back to the
            // targets its rule allows, each added in the order drawn while it may be, and topped up to the fewest it takes.
            if together.is_none() {
                Answer::Targets(chosen)
            } else {
                let parts = together.as_slice();
                let mut kept: Vec<Target> = Vec::new();
                for t in chosen {
                    let verdict = cantrip_engine::select::judge_targets(game, parts, request.player, *source, candidates, &kept);
                    if candidates.iter().position(|&c| c == t).is_some_and(|i| verdict.addable.get(i)) {
                        kept.push(t);
                    }
                }
                if kept.len() < *least as usize {
                    kept = cantrip_engine::select::first_targets(game, together.as_ref(), request.player, *source, candidates, *least as usize).unwrap_or_default();
                }
                Answer::Targets(kept)
            }
        }
        RequestKind::ChooseModes { eligible, count, .. } => Answer::Modes(random.some(eligible, *count as usize)),
        RequestKind::ChooseColors { choices, count, .. } => {
            let colors: Vec<Color> = choices.iter().collect();
            Answer::Colors((0..*count).map(|_| colors[random.below(colors.len())]).collect())
        }
        RequestKind::May { .. } => {
            if random.coin() {
                Answer::Yes
            } else {
                Answer::No
            }
        }
        // A declaration an effect makes requirements of (CR 508.1d): the engine's own, which obeys them.
        RequestKind::DeclareAttackers { requirements, players, .. } if requirements.most > 0 && !players.is_empty() => {
            Answer::Attack(requirements.suggested.clone())
        }
        RequestKind::DeclareAttackers { eligible, players, requirements, .. } => {
            let n = random.below(eligible.len() + 1);
            let attackers = random.some(eligible, n);
            // A player a restriction forbids the creature to attack (CR 508.1c) is not chosen; a creature forbidden every
            // player stays home.
            Answer::Attack(
                attackers
                    .into_iter()
                    .filter_map(|a| {
                        let open: Vec<PlayerId> = players.iter().copied().filter(|&p| !requirements.bars(a, Target::Player(p))).collect();
                        (!open.is_empty()).then(|| (a, Target::Player(open[random.below(open.len())])))
                    })
                    .collect(),
            )
        }
        RequestKind::AssignCombatDamage { divisions } => Answer::Shares(DamageDivision::suggested_shares(divisions)),
        // A declaration an effect makes requirements of (CR 509.1c): the engine's own, which obeys them.
        RequestKind::DeclareBlockers { requirements, .. } if requirements.most > 0 => Answer::Block(requirements.suggested.clone()),
        RequestKind::DeclareBlockers { allowed, requirements, .. } => {
            let mut blocks: Vec<(ObjectId, ObjectId)> = Vec::new();
            for pair in random.some(allowed, allowed.len()) {
                if random.coin() && !blocks.iter().any(|b| b.0 == pair.0) {
                    blocks.push(pair);
                }
            }
            // Menace: an attacker blocked by fewer than it requires is not blocked at all.
            let once: Vec<ObjectId> = blocks
                .iter()
                .map(|b| b.1)
                .filter(|&a| u32::try_from(blocks.iter().filter(|b| b.1 == a).count()).unwrap_or(u32::MAX) < requirements.limits_of(a).0)
                .collect();
            blocks.retain(|b| !once.contains(&b.1));
            // "Can't be blocked by more than one creature": the blockers past the most it allows are left out.
            let mut kept: Vec<(ObjectId, ObjectId)> = Vec::new();
            for b in blocks {
                if u32::try_from(kept.iter().filter(|k| k.1 == b.1).count()).unwrap_or(u32::MAX) < requirements.limits_of(b.1).1 {
                    kept.push(b);
                }
            }
            Answer::Block(kept)
        }
        // The whole total on the first candidate.
        RequestKind::ChooseDividedTargets { candidates, total, .. } => {
            Answer::Divided(if *total > 0 && !candidates.is_empty() { vec![(candidates[0], *total)] } else { Vec::new() })
        }
        RequestKind::ChoosePlayer { candidates, .. } => Answer::Player(candidates[random.below(candidates.len())]),
        RequestKind::CollectEvidence { cards, .. } => Answer::Cards(cards.clone()),
        RequestKind::Craft { cards, minimum, .. } => Answer::Cards(cards[..*minimum as usize].to_vec()),
        // As many as there are from each pool in turn.
        RequestKind::RemoveCounters { pools, total, .. } => {
            let mut left = *total as u16;
            let mut taken = Vec::new();
            for p in pools {
                let n = p.count.min(left);
                if n > 0 {
                    taken.push((p.object, p.kind, n));
                    left -= n;
                }
            }
            Answer::Counters(taken)
        }
        RequestKind::TapForPower { purpose: Purpose::TapToReduce, .. } => Answer::Cards(Vec::new()),
        RequestKind::TapForPower { cards, powers, minimum, .. } => {
            // Everything with positive power that may be tapped, the most that can be counted: always enough if anything is.
            let _ = minimum;
            Answer::Cards(cards.iter().zip(powers).filter(|&(_, &p)| p > 0).map(|(&c, _)| c).collect())
        }
        RequestKind::DiscardToHandSize { count, hand } => Answer::Cards(random.some(hand, *count as usize)),
        RequestKind::Mulligan { .. } => Answer::No,
        RequestKind::ChooseStartingPlayer { candidates } => Answer::Player(candidates[random.below(candidates.len())]),
        RequestKind::DistributeCards { cards, destinations, .. } => {
            Answer::Placements { placements: greedy_distribution(cards, destinations), shuffle: false }
        }
        RequestKind::SearchLibrary { rule, .. } => {
            let eligible = first_choices(game, request);
            let n = match rule.count {
                Count::UpTo(n) => random.below(n.max(1) as usize + 1),
                Count::AnyNumber => random.below(eligible.len() + 1),
                Count::Exactly(n) | Count::AsManyAsCan(n) => (n as usize).min(eligible.len()),
            };
            Answer::Cards(fitting(game, request, random.some(&eligible, n)))
        }
        RequestKind::OrderTriggers { sources, .. } => {
            let all: Vec<u32> = (0..count_u32(sources.len())).collect();
            Answer::Order(random.some(&all, all.len()))
        }
        RequestKind::ChooseReplacement { sources, .. } => Answer::Index(random.below(sources.len()) as u8),
        RequestKind::ChooseName { suggestion, kind, .. } => Answer::Name(default_name(game.cards, *kind, *suggestion)),
        RequestKind::ChooseSubtype { choices, .. } => Answer::Subtype(choices[random.below(choices.len())]),
        RequestKind::ChooseNumber { least, most, .. } => {
            Answer::Announce(least + random.below((most - least + 1) as usize) as u32)
        }
    }
}

/// A decklist file of `decks/`: `N Name` lines, `#` comments.
pub fn load_deck(path: &std::path::Path) -> Vec<String> {
    let text = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let mut cards = Vec::new();
    for line in text.lines().map(str::trim).filter(|l| !l.is_empty() && !l.starts_with('#')) {
        let (n, name) = line.split_once(' ').expect("N Name");
        for _ in 0..n.parse::<usize>().expect("a count") {
            cards.push(name.trim().to_string());
        }
    }
    cards
}

/// How one game went.
pub struct Played {
    pub decisions: u32,
    pub turns: u32,
    pub finished: bool,
    /// Time spent inside the engine answering, apart from the seat choosing.
    pub engine: std::time::Duration,
}

/// Plays a game to its end from the deal, stopping after `limit` decisions.
pub fn play(game: &mut Game, random: &mut Random, limit: u32) -> Played {
    let mut decisions = 0;
    let mut engine = std::time::Duration::ZERO;
    while let Some(request) = game.pending().cloned() {
        if decisions >= limit {
            break;
        }
        let answer = decide(game, random, &request);
        let t = std::time::Instant::now();
        let result = game.answer(request.id, request.player, answer.clone());
        engine += t.elapsed();
        if let Err(Rejected(why)) = result {
            panic!("the engine refused the random seat's answer {answer:?} to {:?}: {why}", request.kind);
        }
        decisions += 1;
    }
    Played { decisions, turns: game.state.turn.number, finished: game.is_over(), engine }
}

/// A random choice of cards cut back to those its question's rule lets be added one after another, in the order drawn ("with
/// total mana value N or less", "with different names").
fn fitting(game: &Game, request: &Request, drawn: Vec<ObjectId>) -> Vec<ObjectId> {
    let mut kept: Vec<ObjectId> = Vec::new();
    for card in drawn {
        let Some((cards, verdict)) = selection_verdict(game, request, &kept) else { break };
        if cards.iter().position(|&c| c == card).is_some_and(|i| verdict.addable.get(i)) {
            kept.push(card);
        }
    }
    kept
}
