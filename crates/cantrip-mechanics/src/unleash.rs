//! Unleash (CR 702.98a), two static abilities: "You may have this permanent enter with an additional +1/+1 counter on it"
//! and "This permanent can't block as long as it has a +1/+1 counter on it." Bloodfray Giant, Carnival Hellsteed, Dead
//! Reveler, Gore-House Chainwalker, Rakdos Cackler, Rakdos Drake, Spawn of Rix Maadi, Splatter Thug and Thrill-Kill
//! Assassin print it.
//!
//! A card with unleash has `enters: Some(unleash::enters)` and lists `CANT_BLOCK` among its `statics`.

use crate::statics::itself;
use cantrip_api::*;

const UNLEASH: Question = Question::new(
    "Unleash",
    "have it enter with a +1/+1 counter?",
    "It can't block as long as it has a +1/+1 counter on it",
    "Enter with a +1/+1 counter",
    "Enter without a counter",
);

/// "You may have this creature enter with an additional +1/+1 counter on it" (CR 614.1c, 122.6): the counter is on it as it
/// enters, so it is never without it on the battlefield.
pub fn enters(g: &mut dyn Game, entering: &mut Entering) -> Result<(), Suspend> {
    if g.ask(entering.controller, RequestKind::May { source: entering.object, purpose: Purpose::Ask(&UNLEASH) })? == Answer::Yes {
        entering.counters[CounterKind::PlusOne as usize] += 1;
    }
    Ok(())
}

/// Only while it has a +1/+1 counter on it, whatever put it there (CR 613.1f, asked again whenever characteristics are
/// computed).
fn itself_with_a_plus_counter(q: &dyn LayerQuery, source: ObjectId, affected: ObjectId, so_far: &Chars) -> bool {
    itself(q, source, affected, so_far) && q.state().obj(source).counters(CounterKind::PlusOne) > 0
}

fn cant_block(_: &dyn LayerQuery, _: ObjectId, chars: &mut Chars) {
    chars.keywords = chars.keywords.with(Keyword::CantBlock);
}

pub const CANT_BLOCK: StaticDef =
    static_ability(Layer::Ability, itself_with_a_plus_counter, cant_block, "This creature can't block as long as it has a +1/+1 counter on it.");
