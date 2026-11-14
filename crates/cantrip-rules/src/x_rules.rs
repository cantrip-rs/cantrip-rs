//! Printed X definitions and limits (CR 107.3, 601.2b). Ordinary X retains its existing announcement path.
use crate::{Cx, R, Stop};
use cantrip_api::*;
use cantrip_state::XCost;

pub(crate) enum Values {
    One(Option<u32>),
    Range(std::ops::RangeInclusive<u32>),
    Players(std::vec::IntoIter<u32>),
}
impl Iterator for Values {
    type Item = u32;
    fn next(&mut self) -> Option<u32> {
        match self {
            Self::One(n) => n.take(),
            Self::Range(r) => r.next(),
            Self::Players(v) => v.next(),
        }
    }
}
/// Announcement search bounds limit freely chosen X, not a value the text defines.
/// Player-defined values are sparse (different graveyard counts), not every number between them.
pub(crate) fn values(
    q: &dyn Query,
    asker: &Asker,
    rule: Option<XRule>,
    targets: &[Target],
    least: u32,
    search_most: u32,
) -> Values {
    match rule {
        None => Values::Range(least..=search_most),
        Some(XRule::AtMost(value)) => {
            Values::Range(least..=search_most.min(value(q, asker, targets)))
        }
        Some(XRule::Is(value)) => {
            Values::One(Some(value(q, asker, targets)).filter(|&n| n >= least))
        }
        Some(XRule::OfPlayer { players, value }) => {
            let mut ns: Vec<_> = q
                .state()
                .apnap()
                .filter(|&p| players(q, asker, p))
                .map(|p| value(q, asker, p))
                .filter(|&n| n >= least)
                .collect();
            ns.sort_unstable();
            ns.dedup();
            Values::Players(ns.into_iter())
        }
    }
}
pub(crate) fn least(
    q: &dyn Query,
    asker: &Asker,
    rule: Option<XRule>,
    targets: &[Target],
    minimum: u32,
) -> Option<u32> {
    values(q, asker, rule, targets, minimum, minimum).next()
}
pub(crate) fn upper(q: &dyn Query, asker: &Asker, rule: Option<XRule>, targets: &[Target]) -> u32 {
    match rule {
        Some(XRule::AtMost(value)) => value(q, asker, targets),
        _ => u32::MAX,
    }
}
pub(crate) fn defines(rule: Option<XRule>) -> bool {
    matches!(rule, Some(XRule::Is(_) | XRule::OfPlayer { .. }))
}
/// A text-defined X is not a freely announced number. A player's count is read once as the spell is cast.
/// The choice of player is not a target, and uses the existing typed player-choice protocol.
pub(crate) fn choose(
    cx: &mut Cx,
    asker: &Asker,
    rule: Option<XRule>,
    targets: &[Target],
    minimum: u32,
    cap: u32,
    paid: Option<&XCost>,
) -> R<Option<u32>> {
    match rule {
        Some(XRule::Is(value)) => {
            let n = value(cx, asker, targets);
            if n < minimum || n > cap {
                return Err(Stop::Cancel);
            };
            Ok(Some(n))
        }
        Some(XRule::OfPlayer { players, value }) => {
            let candidates: Vec<_> = cx
                .s()
                .apnap()
                .filter(|&p| players(cx, asker, p))
                .filter(|&p| {
                    let n = value(cx, asker, p);
                    n >= minimum
                        && n <= cap
                        && paid.is_none_or(|paid| {
                            crate::actions::defined_x_payable(
                                cx,
                                asker.controller,
                                asker.source,
                                paid,
                                n,
                            )
                        })
                })
                .collect();
            let player = match candidates.as_slice() {
                [] => return Err(Stop::Cancel),
                [p] => *p,
                _ => match cx.ask_core(
                    asker.controller,
                    RequestKind::ChoosePlayer {
                        source: asker.source,
                        candidates,
                        verb: "Define X",
                        amount: 0,
                        subject: None,
                    },
                    true,
                )? {
                    Answer::Player(p) => p,
                    _ => return Err(Stop::Cancel),
                },
            };
            Ok(Some(value(cx, asker, player)))
        }
        _ => Ok(None),
    }
}
