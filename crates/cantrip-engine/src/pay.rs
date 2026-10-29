//! Paying from the pool without being told (`AutoPay`): the scenario runner's payment, which the AI player
//! (`cantrip_ai::pay::pay`) pays with first too. `auto_pay` is told what a line says (`PayPlan`) and answers one payment
//! question at a time.

use std::collections::VecDeque;

use crate::*;

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
        let mut wanted = plan.convoke.pop_front().unwrap();
        // A name with a comma in it (`Eirdu, Carrier of Dawn`) arrives split at the comma: when the piece alone names no
        // creature offered, the pieces after it are joined to it until they do.
        let offered_named = |name: &str| {
            options.iter().any(|o| {
                o.kind == ActionKind::TapToPay && o.source.is_some_and(|s| game.known_name(s).eq_ignore_ascii_case(name))
            })
        };
        if !offered_named(&wanted)
            && let Some(joined) = (1..=plan.convoke.len()).find_map(|n| {
                let name = std::iter::once(wanted.as_str()).chain(plan.convoke.iter().take(n).map(String::as_str)).collect::<Vec<_>>().join(", ");
                offered_named(&name).then_some((n, name))
            })
        {
            plan.convoke.drain(..joined.0);
            wanted = joined.1;
        }
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
