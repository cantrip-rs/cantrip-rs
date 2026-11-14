use cantrip_api::*;
use cantrip_mechanics::{effects, triggers};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    // Survival is an ability word (CR 207.2c). "This ability triggers only once" has no numbered rule: it stops triggering for this
    // object once it has triggered (a new object, CR 400.7, has not). "If this creature is tapped" is an intervening "if" (CR 603.4),
    // so a Cheerleader that is untapped at the beginning of the second main phase has not triggered and has not used it up.
    // "Put a flying counter on it" (CR 122.1b) is the source itself: nothing once it has left the battlefield (CR 400.7).
    abilities: &[triggered(
        triggers::YOUR_SECOND_MAIN_PHASE,
        effects::counters_on_self::<{ CounterKind::Flying as usize }, 1>,
        "Survival \u{2014} At the beginning of your second main phase, if this creature is tapped, put a flying counter on it. This ability triggers only once.",
    )
    .condition(triggers::this_is_tapped)
    .triggers_only_once()],
    ..oracle::ACROBATIC_CHEERLEADER
};
