use cantrip_api::*;
use cantrip_mechanics::library;
use cantrip_mechanics::triggers::{THIS_ENTERS, YOUR_END_STEP};
use cantrip_mechanics::{creatures_you_control, effects, mana};
use cantrip_oracle as oracle;

/// "If you control four or more creatures" (CR 603.4): an intervening "if", asked as the end step begins and again as the
/// ability resolves (the ruling). The creatures are counted as they are now, through the layers.
fn you_control_four_or_more_creatures(q: &dyn Query, asker: &Asker) -> bool {
    creatures_you_control(q, asker.controller) >= 4
}

/// "{T}: Add {G}." and "{T}: Add {G} for each creature you control." (CR 106.1, 605.1a): Itlimoc, Cradle of the Sun.
pub static BACK: CardDef = CardDef {
    abilities: &[
        mana_ability(TAP, add(Mana::G), "{T}: Add {G}."),
        mana_ability(TAP, add_by(mana::add_green_for_each_creature), "{T}: Add {G} for each creature you control."),
    ],
    ..oracle::ITLIMOC_CRADLE_OF_THE_SUN
};

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            THIS_ENTERS,
            library::look_for_a_creature_rest_on_the_bottom_in_any_order::<4>,
            "When Growing Rites of Itlimoc enters, look at the top four cards of your library. You may reveal a creature card from among them and put it into your hand. Put the rest on the bottom of your library in any order.",
        ),
        triggered(
            YOUR_END_STEP,
            effects::transform_self,
            "At the beginning of your end step, if you control four or more creatures, transform Growing Rites of Itlimoc.",
        )
        .condition(you_control_four_or_more_creatures),
    ],
    back: Some(&BACK),
    ..oracle::GROWING_RITES_OF_ITLIMOC
};
