use cantrip_api::*;
use cantrip_mechanics::triggers::{ANOTHER_CREATURE_YOU_CONTROL_DIES, THIS_ENTERS};
use cantrip_mechanics::{effects, tokens};
use cantrip_oracle as oracle;

/// "Whenever another creature you control dies" (CR 700.4, 603.10a): each creature that dies triggers it separately, a token
/// included, and the Rat token this makes counts as soon as it dies.
pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            THIS_ENTERS,
            tokens::create_rats_that_cant_block::<1>,
            "When this creature enters, create a 1/1 black Rat creature token with \"This token can't block.\"",
        ),
        triggered(
            ANOTHER_CREATURE_YOU_CONTROL_DIES,
            effects::plus_one_counter_on_this,
            "Whenever another creature you control dies, put a +1/+1 counter on this creature.",
        ),
    ],
    ..oracle::VORACIOUS_VERMIN
};

pub static EXTRA: &[&CardDef] = &[&tokens::RAT_CANT_BLOCK];
