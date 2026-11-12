//! Target choice changes the decision maker, never the spell/ability controller.
use cantrip_api::*;
use crate::{Cx, R, Stop, actions, resolve::Given};

pub(crate) fn players(q: &dyn Query, asker: &Asker, choice: TargetChooser) -> Vec<PlayerId> {
    let proposed = match choice { TargetChooser::Player(f) => vec![f(q, asker)], TargetChooser::OneOf(f) => f(q, asker) };
    let mut out = Vec::new();
    for player in proposed {
        if q.state().seats().any(|p| p == player) && !q.state().player(player).lost && !out.contains(&player) { out.push(player); }
    }
    out
}

/// An opponent must be able to complete this slot alone. Combining candidates
/// from several opponents would incorrectly make two-target slots payable.
pub(crate) fn eligible(given: &Given<'_>, asker: &Asker, spec: &TargetSpec, counted: u32) -> Vec<PlayerId> {
    let Some(choice) = spec.chooser else { return Vec::new() };
    players(given, asker, choice).into_iter().filter(|&player| {
        let q = Given { chooser: Some(player), ..*given };
        let candidates = actions::target_candidates_as(&q, asker, spec);
        let (candidates, together) = actions::within_together(&q, asker, spec, counted, candidates);
        actions::enough_together(&q, asker, &together, &candidates, spec.bounds(counted).0)
    }).collect()
}

pub(crate) fn choose(cx: &mut Cx, asker: &Asker, spec: &TargetSpec, eligible: Vec<PlayerId>, default: PlayerId, cancel: bool) -> R<Option<PlayerId>> {
    if spec.chooser.is_none() { return Ok(Some(default)); }
    match eligible.as_slice() {
        [] => Ok(None),
        [one] => Ok(Some(*one)),
        _ => match cx.ask_core(asker.controller, RequestKind::ChoosePlayer { source: asker.source, candidates: eligible, verb: "choose targets", amount: 0, subject: None }, cancel)? {
            Answer::Player(player) => Ok(Some(player)),
            _ => Err(Stop::Cancel),
        },
    }
}
