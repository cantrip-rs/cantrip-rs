use cantrip_api::*;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_mechanics::{effects, evoke, spent, targets};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    ways: &[evoke::way(Cost::mana(mana("{W/B}{W/B}")), "Evoke {W/B}{W/B}")],
    abilities: &[
        evoke::sacrifice_trigger("When Emptiness enters, if its evoke cost was paid, its controller sacrifices it."),
        triggered(
            THIS_ENTERS,
            effects::return_first_target_to_battlefield,
            "When this creature enters, if {W}{W} was spent to cast it, return target creature card with mana value 3 or less from your graveyard to the battlefield.",
        )
        .targets(&[targets::CREATURE_CARD_WITH_MANA_VALUE_3_OR_LESS_IN_YOUR_GRAVEYARD])
        .condition(spent::two::<{ Mana::W as u8 }>),
        triggered(
            THIS_ENTERS,
            effects::minus_one_counters_on_target::<3>,
            "When this creature enters, if {B}{B} was spent to cast it, put three -1/-1 counters on up to one target creature.",
        )
        .targets(&[targets::CREATURE.between(0, 1)])
        .condition(spent::two::<{ Mana::B as u8 }>),
    ],
    ..oracle::EMPTINESS
};
