use cantrip_api::*;
use cantrip_mechanics::effects::plus_one_counter_on_target;
use cantrip_mechanics::targets::CREATURE_YOU_CONTROL;
use cantrip_mechanics::tokens;
use cantrip_mechanics::triggers::{ANOTHER_CREATURE_YOU_CONTROL_ENTERS, THIS_ENTERS};
use cantrip_oracle as oracle;

/// Alliance (the rulings): its own Mutant token entering is another creature entering under its controller, so it triggers
/// the second ability once the first has resolved. The target is chosen as the trigger goes on the stack (CR 603.3d), among the
/// creatures its controller has then, the creature that entered included.
pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(THIS_ENTERS, tokens::create_a_mutant, "When this creature enters, create a 2/2 red Mutant creature token."),
        triggered(
            ANOTHER_CREATURE_YOU_CONTROL_ENTERS,
            plus_one_counter_on_target,
            "Alliance \u{2014} Whenever another creature you control enters, put a +1/+1 counter on target creature you control.",
        )
        .targets(&[CREATURE_YOU_CONTROL]),
    ],
    ..oracle::MIGHTY_MUTANIMALS
};

pub static EXTRA: &[&CardDef] = &[&tokens::MUTANT];
