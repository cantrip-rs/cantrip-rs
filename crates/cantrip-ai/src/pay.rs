//! Paying for a spell or ability without a policy step (docs/ai-player-plan.md, "Answers"): mana payment is not a
//! decision the policy learns. `auto_pay` is the scenario runner's payment (moved here from `cantrip-script`, which
//! calls it), told what a line says (`PayPlan`); `pay` is what the AI player pays with, `auto_pay` first and then the
//! mana abilities a payment needs.

use std::collections::VecDeque;

use cantrip_engine::*;

/// Pool mana a payment spends: its kind, its restriction (CR 106.6), and the name of the source that produced it, for
/// mana of one kind from several sources.
pub type Spend = (Mana, Option<ManaRestriction>, Option<String>);

/// What a payment is told to do beyond paying what is owed from the pool: the X or kicker count to announce, the pool
/// mana to spend first, the creatures Convoke taps (in the order the payment asks, CR 702.51a) and how many cards Delve
/// exiles from the graveyard, the first offered each time (CR 702.66a).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PayPlan {
    pub x: Option<u16>,
    pub spend: VecDeque<Spend>,
    pub convoke: VecDeque<String>,
    pub delve: u32,
}

impl PayPlan {
    pub fn new(x: Option<u16>, spend: VecDeque<Spend>) -> PayPlan {
        PayPlan {
            x,
            spend,
            convoke: VecDeque::new(),
            delve: 0,
        }
    }
}

/// Whether `mana` pays any of what is `owed`.
pub fn pays(owed: &ManaCost, mana: Mana) -> bool {
    owed.generic > 0
        || owed.symbols[mana as usize] > 0
        || owed
            .hybrid
            .iter()
            .flatten()
            .any(|(a, b)| *a == mana || *b == mana)
}

/// One answer towards paying a `PayCosts` request from the pool, as `plan` says (`AutoPay`): the X announced, then
/// Convoke and Delve, then `Spend:`, then any pool mana that pays what is owed, then confirming. `None` where it has
/// nothing to answer with (more mana is needed); an error where the plan names what is not offered.
pub fn auto_pay(
    game: &Game,
    request: &Request,
    plan: &mut PayPlan,
) -> Result<Option<Answer>, String> {
    let RequestKind::PayCosts {
        remaining,
        options,
        announcing,
        x,
        most,
        least,
        ..
    } = &request.kind
    else {
        return Ok(None);
    };
    let offers = |kind: ActionKind| options.iter().find(|o| o.kind == kind);
    if *announcing {
        // X, or how many times it is kicked, announced in one answer within what the question allows; a plan that says
        // none announces the value the question starts at, as a page does: the least it may be (an X that can't be 0 is
        // 1), or the X a reduction already announced pays for (`ExileFromHandForLess`).
        let want = plan.x.map_or(*x, u32::from);
        return Ok((*least..=*most).contains(&want).then_some(Answer::Announce(want)));
    }
    // Convoke taps the creatures it names, one per question, and Delve exiles that many cards from the graveyard, the
    // first offered each time: what pays the cost instead of mana (CR 601.2h).
    if !plan.convoke.is_empty() && options.iter().any(|o| o.kind == ActionKind::TapToPay) {
        let wanted = plan.convoke.pop_front().unwrap();
        let tap = options.iter().find(|o| {
            o.kind == ActionKind::TapToPay
                && o.source
                    .is_some_and(|s| game.known_name(s).eq_ignore_ascii_case(&wanted))
        });
        return match tap {
            Some(o) => Ok(Some(Answer::Act(Action::of(o)))),
            None => Err(format!(
                "Convoke: names {wanted}, but the creatures offered are {:?}",
                options
                    .iter()
                    .filter(|o| o.kind == ActionKind::TapToPay)
                    .filter_map(|o| o.source)
                    .map(|s| game.known_name(s))
                    .collect::<Vec<_>>()
            )),
        };
    }
    if plan.delve > 0
        && let Some(o) = options.iter().find(|o| o.kind == ActionKind::ExileToPay)
    {
        plan.delve -= 1;
        return Ok(Some(Answer::Act(Action::of(o))));
    }
    let assign: Vec<&ActionOption> = options
        .iter()
        .filter(|o| o.kind == ActionKind::AssignMana)
        .collect();
    if !assign.is_empty()
        && let Some(wanted) = plan.spend.pop_front()
    {
        let from = |o: &ActionOption| o.source.map(|s| game.known_name(s).to_string());
        return match assign.iter().find(|o| {
            (o.mana, o.restriction) == (Some(wanted.0), wanted.1)
                && from(o).map(|n| n.to_ascii_lowercase())
                    == wanted.2.as_ref().map(|n| n.to_ascii_lowercase())
        }) {
            Some(o) => Ok(Some(Answer::Act(Action::of(o)))),
            None => Err(format!(
                "Spend: names {wanted:?}, but the pool mana offered is {:?}",
                assign.iter().map(|o| (o.mana, from(o))).collect::<Vec<_>>()
            )),
        };
    }
    if !remaining.is_zero()
        && let Some(o) = assign
            .iter()
            .find(|o| o.mana.is_some_and(|m| pays(remaining, m)))
    {
        return Ok(Some(Answer::Act(Action::of(o))));
    }
    if let Some(o) = offers(ActionKind::ConfirmCasting) {
        return Ok(Some(Answer::Act(Action::of(o))));
    }
    Ok(None)
}

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

/// The mana ability to activate next towards `owed`. The colored and hybrid symbols owed are matched to sources first,
/// each its own source (`cover`), and the first of them is tapped for its symbol: so the generic part is never paid with
/// the only source of a color still owed (an Island tapped for {1} of {1}{U}) and a dual land is not spent on a color a
/// basic land adds. With only generic mana owed, or no matching, the first ability that adds anything owed.
fn tap(game: &Game, options: &[ActionOption], owed: &ManaCost) -> Option<Action> {
    let candidates: Vec<(&ActionOption, Vec<Mana>)> = options
        .iter()
        .filter(|o| mana_action(game, o, owed).is_some())
        .map(|o| (o, kinds(game, o)))
        .collect();
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
    options.iter().find_map(|o| mana_action(game, o, owed))
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
