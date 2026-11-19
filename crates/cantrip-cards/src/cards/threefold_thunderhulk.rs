use cantrip_api::*;
use cantrip_mechanics::triggers::THIS_ENTERS_OR_ATTACKS;
use cantrip_mechanics::{effects, targets, tokens};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    enters: Some(effects::enters_with_plus_one_counters::<3>),
    abilities: &[
        triggered(
            THIS_ENTERS_OR_ATTACKS,
            |g, item| tokens::create_equal_to_its_power(g, item, &tokens::GNOME),
            "Whenever this creature enters or attacks, create a number of 1/1 colorless Gnome artifact creature tokens equal to its power.",
        ),
        ability(
            cost("{2}").and(sacrifice(1, targets::filters::another_artifact)),
            effects::plus_one_counter_on_this,
            "{2}, Sacrifice another artifact: Put a +1/+1 counter on this creature.",
        ),
    ],
    ..oracle::THREEFOLD_THUNDERHULK
};

pub static EXTRA: &[&CardDef] = &[&tokens::GNOME];
