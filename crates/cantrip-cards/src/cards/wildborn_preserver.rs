use cantrip_api::*;
use cantrip_mechanics::{effects, triggers};
use cantrip_oracle as oracle;

const PAY_X: Question = Question::new(
    "PutPlusOneCounters",
    "pay {X}?",
    "When you do, put X +1/+1 counters on this creature",
    "Pay {cost}",
    "Don't pay",
)
.no_default();

/// "Whenever another non-Human creature you control enters, you may pay {X}. When you do, ..." X is the payer's to choose as
/// they pay it (CR 107.3f), 0 included; the reflexive ability (CR 603.12) triggers carrying X, and declining triggers nothing.
fn enters(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::may_pay_x_then_when_you_do(g, item, &CARD, &PAY_X, Target::None)
}

/// "Whenever another non-Human creature you control enters" (CR 603.6a, 205.3m): a creature other than this one, under your
/// control as it entered, that is not a Human (a token too), the type read through the words of this card's text (CR 612), so a
/// changeling is a Human and sets nothing off.
fn another_non_human_enters(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    match event {
        Event::ZoneChange { new, .. } if !q.chars(*new).has_subtype(q.words(me.source).subtype(Subtype::Human)) => {
            triggers::another_creature_you_control_enters(q, me, event)
        }
        _ => None,
    }
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        when_asked(EventMask::ZONE_CHANGE, another_non_human_enters),
        enters,
        "Whenever another non-Human creature you control enters, you may pay {X}. When you do, put X +1/+1 counters on this creature.",
    )],
    delayed: &[delayed_trigger(effects::x_plus_one_counters_on_this, "When you do, put X +1/+1 counters on this creature.")],
    ..oracle::WILDBORN_PRESERVER
};
