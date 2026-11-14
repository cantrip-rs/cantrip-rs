use cantrip_api::*;
use cantrip_mechanics::{effects, targets, triggers};
use cantrip_oracle as oracle;

/// "Eerie \u{2014} Whenever an enchantment you control enters and whenever you fully unlock a Room, put a +1/+1 counter on target
/// creature." Eerie is an ability word (CR 207.2c): one triggered ability that two events trigger (CR 603.2). The target is
/// chosen as the ability goes on the stack (CR 603.3d) and judged again on resolution (CR 608.2b).
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::EERIE,
        effects::plus_one_counter_on_target,
        "Eerie \u{2014} Whenever an enchantment you control enters and whenever you fully unlock a Room, put a +1/+1 counter on target creature.",
    )
    .targets(&[targets::CREATURE])],
    ..oracle::OPTIMISTIC_SCAVENGER
};
