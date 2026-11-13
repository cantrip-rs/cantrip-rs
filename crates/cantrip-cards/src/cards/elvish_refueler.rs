use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;

/// "During your turn, as long as you haven't activated an exhaust ability this turn, you may activate exhaust abilities as though
/// they haven't been activated" is the core's `exhaust_as_though_unactivated` (CR 702.177b). "Put a +1/+1 counter on this
/// creature" is its own exhaust ability.
pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        cost("{1}{G}"),
        effects::plus_one_counter_on_this,
        "Exhaust \u{2014} {1}{G}: Put a +1/+1 counter on this creature. (Activate each exhaust ability only once.)",
    )
    .exhaust()],
    exhaust_as_though_unactivated: true,
    ..oracle::ELVISH_REFUELER
};
