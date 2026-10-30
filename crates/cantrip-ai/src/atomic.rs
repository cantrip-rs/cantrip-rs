//! The AI's own requests made atomic: a cast or activation is one choice, paid at once
//! by the autotapper (`crate::pay`), never given up part way, and mana is made only by paying.
//!
//! - **No cancelling.** A request that may be cancelled (a cast being announced, a payment) is answered without the
//!   cancel: `may_cancel` cleared and the "cancel casting" options taken out, so the factoring never offers its `Cancel`
//!   token. The one exception is a payment the autotapper cannot finish (`pay::pay` has nothing to answer with): there
//!   the cancel is the only answer left, and stays (the factoring then has nothing else, so it is no choice either).
//!   A choice of cards towards a total that can be out of reach (tapping creatures for power, collecting evidence,
//!   crafting) keeps its cancel, as the engine may leave it the only answer.
//!   The engine is untouched: a person may cancel as ever (CR 601.2, 733); the AI simply never chooses to.
//! - **No mana abilities at priority.** Activating a mana ability on its own is taken out of the AI's priority options:
//!   mana is made only as part of paying (`pay`). The mana abilities whose activation does more than add mana are listed
//!   by `cantrip-probe mana-report`, so what the AI gives up is known.
//!
//! What a cast or activation can't be paid for in a line of the search is found where the line plays it: the payment's
//! request has nothing to answer with, and the search takes the edge as refused (`search`). The engine offers a cast at
//! priority only where its mana advice says it can be paid (`actions::priority_options`), so such a line is rare.
//!
//! Only the AI's copy of a request is changed (the one it chooses from): the engine still judges every answer.

use cantrip_engine::*;

/// Whether priority option `option` activates a mana ability (CR 605.1a): an ability of its source whose definition adds
/// mana, or a player's own mana ability.
pub fn is_mana_ability(game: &Game, option: &ActionOption) -> bool {
    if option.kind != ActionKind::ActivateAbility {
        return false;
    }
    if option.index >= PLAYER_MANA_ABILITY {
        return true;
    }
    let Some(source) = option.source else {
        return false;
    };
    game.chars(source)
        .abilities()
        .nth(option.index as usize)
        .and_then(|a| game.activated_ability(a))
        .is_some_and(|d| d.is_mana_ability())
}

/// Whether `answer` is a priority's activation of a mana ability on its own (as `is_mana_ability` judges the option).
pub fn activates_mana_ability(game: &Game, request: &Request, answer: &Answer) -> bool {
    match (&request.kind, answer) {
        (RequestKind::Priority { .. }, Answer::Act(a)) => {
            is_mana_ability(game, &ActionOption { source: a.source, index: a.index, ..ActionOption::simple(a.kind) })
        }
        _ => false,
    }
}

/// Whether an answer cancels what is being cast or activated.
pub fn cancels(answer: &Answer) -> bool {
    match answer {
        Answer::Cancel => true,
        Answer::Act(a) => a.kind == ActionKind::CancelCasting,
        _ => false,
    }
}

/// Whether `request` is a payment the autotapper cannot finish: one it would answer with nothing.
pub fn unpayable(game: &Game, request: &Request) -> bool {
    matches!(&request.kind, RequestKind::PayCosts { announcing: false, .. }) && crate::pay::pay(game, request, None).is_none()
}

/// The AI's copy of `request`, atomic (see the module).
pub fn filter(game: &Game, request: &Request) -> Request {
    let mut r = request.clone();
    let payable = match &r.kind {
        RequestKind::PayCosts { announcing, .. } => *announcing || crate::pay::pay(game, request, None).is_some(),
        _ => true,
    };
    match &mut r.kind {
        RequestKind::Priority { options } => {
            options.retain(|o| o.kind != ActionKind::CancelCasting && !is_mana_ability(game, o));
        }
        RequestKind::PayCosts { options, .. } | RequestKind::UnlessPays { options, .. } if payable => {
            options.retain(|o| o.kind != ActionKind::CancelCasting);
        }
        _ => {}
    }
    // A choice of cards towards a total (tapping creatures for power, collecting evidence, crafting) can be out of reach,
    // and the cancel its only answer: those keep it.
    let total = matches!(r.kind, RequestKind::TapForPower { .. } | RequestKind::CollectEvidence { .. } | RequestKind::Craft { .. });
    if payable && !total {
        r.may_cancel = false;
    }
    r
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_cancel_is_an_answer_that_cancels() {
        assert!(cancels(&Answer::Cancel));
        assert!(cancels(&Answer::Act(Action::of(&ActionOption::simple(ActionKind::CancelCasting)))));
        assert!(!cancels(&Answer::Pass));
    }
}
