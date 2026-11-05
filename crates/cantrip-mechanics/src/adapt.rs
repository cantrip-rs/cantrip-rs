//! Adapt N (CR 701.46a): "If this permanent has no +1/+1 counters on it, put N +1/+1 counters on it." Emperor of Bones
//! has "{1}{B}: Adapt 2".

use cantrip_api::*;

/// Adapt `n` the permanent `this`, if it is still the permanent on the battlefield that has no +1/+1 counters.
pub fn adapt(g: &mut dyn Game, this: ObjectId, n: i16) {
    if g.is_on_battlefield(this) && g.obj(this).became.is_none() && g.obj(this).counters(CounterKind::PlusOne) == 0 {
        g.add_counters(this, CounterKind::PlusOne, i64::from(n));
    }
}
