use cantrip_api::*;
use cantrip_mechanics::{discover, statics, triggers};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::YOUR_END_STEP,
        |g, item| {
            discover::discover(g, item.controller, 5)?;
            Ok(())
        },
        "At the beginning of your end step, discover 5.",
    )],
    statics: &[static_ability(
        Layer::Ability,
        statics::your_spells_of_types::<{ statics::ANY_SPELL }>,
        statics::cant_be_countered,
        "Spells you control can't be countered.",
    )
    .everywhere()],
    ..oracle::CHIMIL_THE_INNER_SUN
};
