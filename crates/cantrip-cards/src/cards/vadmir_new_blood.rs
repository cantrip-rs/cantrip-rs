use cantrip_api::*;
use cantrip_mechanics::{effects, statics, triggers};
use cantrip_oracle as oracle;

/// "Whenever you commit a crime, put a +1/+1 counter on Vadmir. This ability triggers only once each turn." (CR 700.13; no
/// numbered rule for "only once each turn"): the second crime of a turn doesn't trigger it. "As long as Vadmir has four or more
/// +1/+1 counters on it, it has menace and lifelink." (CR 122.1, 613.1f) counts only +1/+1 counters and follows them as they
/// come and go.
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::YOU_COMMIT_A_CRIME,
        effects::plus_one_counter_on_this,
        "Whenever you commit a crime, put a +1/+1 counter on this creature. This ability triggers only once each turn. (Targeting opponents, anything they control, and/or cards in their graveyards is a crime.)",
    )
    .triggers_each_turn(1)],
    statics: &[static_ability(
        Layer::Ability,
        statics::itself_with_plus_one_counters::<4>,
        statics::have_lifelink_and_menace,
        "As long as this creature has four or more +1/+1 counters on it, it has menace and lifelink.",
    )],
    ..oracle::VADMIR_NEW_BLOOD
};
