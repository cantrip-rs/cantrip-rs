//! Echo [cost] (CR 702.30a): "At the beginning of your upkeep, if this permanent came under your control since the
//! beginning of your last upkeep, sacrifice it unless you pay [cost]." Acridian, Citanul Centaurs, Cradle Guard,
//! Deepcavern Imp, Flamecore Elemental, Goblin Patrol, Goblin War Buggy, Herald of Serra, Pouncing Jaguar, Rakdos Headliner,
//! Shivan Raptor, Simian Grunts, Skizzik Surger, Uktabi Drake, Viashino Outrider, Vug Lizard and Winding Wurm print it. The
//! Urza block's cards have an echo cost equal to their mana cost (CR 702.30b), written on the card as any other.
//!
//! A card with echo lists `echo!(cost, text)` among its `abilities`, `cost` being the echo cost as a `Cost` (`cost("{1}{G}")`,
//! `discard(1)`, ...) and `text` the ability's text. A cost that is not mana and life alone says what it asks in words:
//! `echo!(cost, question, text)` with the `Question` that asks it (`DISCARD_A_CARD`, `SACRIFICE_TWO_LANDS`); a payment shown
//! as mana and life uses `PAY`.
//!
//! "Came under your control since the beginning of your last upkeep" is asked of the event history, as the ability triggers
//! (CR 603.4): the last time the permanent entered the battlefield or changed hands to its controller, against the last
//! upkeep that controller's began before this one. A permanent that came under your control during that upkeep or after it
//! counts, one that came under it before it does not, so a permanent that has been kept through one upkeep is not asked again.
//! The trigger is checked again as it resolves (CR 603.4): the permanent must still
//! be the same one, under the same controller.

use cantrip_api::*;

#[doc(hidden)]
pub use cantrip_api as api;

/// "Pay [cost], or sacrifice it" for an echo cost of mana and life: `{cost}` is that cost.
pub const PAY: Question =
    Question::new("Echo", "pay {cost}, or sacrifice {subject}?", "Echo: it came under your control since your last upkeep", "Pay {cost}", "Sacrifice it").no_default();

/// "Discard a card, or sacrifice it" (Deepcavern Imp, Rakdos Headliner).
pub const DISCARD_A_CARD: Question = Question::new(
    "EchoDiscard",
    "discard a card, or sacrifice {subject}?",
    "Echo: it came under your control since your last upkeep",
    "Discard a card",
    "Sacrifice it",
)
.no_default();

/// "Sacrifice two lands, or sacrifice it" (Skizzik Surger).
pub const SACRIFICE_TWO_LANDS: Question = Question::new(
    "EchoSacrifice",
    "sacrifice two lands, or sacrifice {subject}?",
    "Echo: it came under your control since your last upkeep",
    "Sacrifice two lands",
    "Sacrifice it",
)
.amount(2)
.no_default();

/// Whether `permanent` came under `player`'s control since the beginning of the last upkeep of theirs there is in the
/// history (CR 702.30a): it entered the battlefield, or an effect gave it to them or gave it back, after that upkeep began;
/// with no upkeep of theirs yet, ever. The history is as it stood just before the event asked about (a trigger sees the
/// history before the event it matches), so the upkeep now beginning is not the last one.
fn came_under_control_since_last_upkeep(q: &dyn Query, permanent: ObjectId, player: PlayerId) -> bool {
    let history = q.history();
    let last_upkeep = history.select(EventQuery::kind(EventMask::STEP_BEGAN).step(Step::Upkeep).in_turn_of(player)).last();
    let arrivals = EventQuery::kind(EventMask(EventMask::ZONE_CHANGE.0 | EventMask::CONTROL_CHANGED.0)).object(permanent);
    let arrivals = last_upkeep.map_or(arrivals, |upkeep| arrivals.after(upkeep.serial));
    history.select(arrivals).any(|r| {
        matches!(r.event, Event::ZoneChange { new, to, .. } if new == permanent && to.is(ZoneKind::Battlefield))
            || matches!(r.event, Event::ControlChanged { controller, .. } if controller == player)
    })
}

fn your_upkeep(q: &dyn Query, source: ObjectId, event: &Event) -> Option<Fired> {
    let controller = q.controller(source);
    match event {
        Event::StepBegan { step: Step::Upkeep } if q.state().turn.active == controller && came_under_control_since_last_upkeep(q, source, controller) => {
            Fired::PLAIN
        }
        _ => None,
    }
}

/// The ability's resolution (CR 702.30a): its controller pays `cost`, or sacrifices the permanent. Nothing happens if it is
/// no longer on the battlefield as the same object under that controller (CR 603.4, 400.7). `question` is what the payer is
/// asked.
pub fn resolve(g: &mut dyn Game, item: &StackItem, cost: Cost, question: &'static Question) -> Result<(), Suspend> {
    let this = item.object;
    if !g.is_on_battlefield(this) || g.obj(this).became.is_some() || g.controller(this) != item.controller {
        return Ok(());
    }
    if !g.may_pay(item.controller, this, this, cost, question)? {
        g.sacrifice(item.controller, &[this]);
    }
    Ok(())
}

/// The triggered ability of echo, its effect `pay` being the card's `resolve` with its cost. Use `echo!`.
pub const fn ability(pay: Effect, text: &'static str) -> AbilityDef {
    triggered(when(EventMask::STEP_BEGAN, your_upkeep), pay, text)
}

/// Echo [cost] (CR 702.30a): `echo!(cost, text)` or `echo!(cost, question, text)`; see the module.
#[macro_export]
macro_rules! echo {
    ($cost:expr, $text:expr) => {
        $crate::echo!($cost, $crate::echo::PAY, $text)
    };
    ($cost:expr, $question:expr, $text:expr) => {{
        fn pay(g: &mut dyn $crate::echo::api::Game, item: &$crate::echo::api::StackItem) -> Result<(), $crate::echo::api::Suspend> {
            $crate::echo::resolve(g, item, $cost, &$question)
        }
        $crate::echo::ability(pay, $text)
    }};
}
