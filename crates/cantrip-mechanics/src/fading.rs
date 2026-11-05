//! Fading N (CR 702.32a): "This permanent enters with N fade counters on it. At the beginning of your upkeep, remove a
//! fade counter from it. If you can't, sacrifice it." Parallax Wave prints it with 5.
//!
//! A card with fading gives `enters::<N>` as its `enters` and lists `UPKEEP` among its `abilities`.

use cantrip_api::*;

const FADE: usize = CounterKind::Fade as usize;

pub fn enters<const N: i16>(_: &mut dyn Game, entering: &mut Entering) -> Result<(), Suspend> {
    entering.counters[FADE] += i64::from(N);
    Ok(())
}

fn your_upkeep(q: &dyn Query, source: ObjectId, event: &Event) -> Option<Fired> {
    match event {
        Event::StepBegan { step: Step::Upkeep } if q.state().turn.active == q.controller(source) => Fired::PLAIN,
        _ => None,
    }
}

/// "Remove a fade counter from it. If you can't, sacrifice it": nothing if it is no longer on the battlefield. The
/// ability's controller is who sacrifices it.
fn fade(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let this = item.object;
    if !g.is_on_battlefield(this) || g.obj(this).became.is_some() {
        return Ok(());
    }
    if g.obj(this).counters(CounterKind::Fade) > 0 {
        g.remove_counters(this, CounterKind::Fade, 1);
        return Ok(());
    }
    g.sacrifice(item.controller, &[this]);
    Ok(())
}

/// `text` names the card.
pub const fn upkeep(text: &'static str) -> AbilityDef {
    triggered(when(EventMask::STEP_BEGAN, your_upkeep), fade, text)
}
