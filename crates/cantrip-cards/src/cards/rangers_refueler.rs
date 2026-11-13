use cantrip_api::*;
use cantrip_mechanics::crew::crew;
use cantrip_mechanics::{effects, triggers};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            triggers::YOU_ACTIVATE_AN_EXHAUST_ABILITY,
            |g, item| effects::draw(g, item, 1),
            "Whenever you activate an exhaust ability, draw a card.",
        ),
        ability(
            cost("{4}"),
            effects::this_vehicle_becomes_artifact_creature_with_plus_one_counters::<1>,
            "Exhaust \u{2014} {4}: This Vehicle becomes an artifact creature. Put a +1/+1 counter on it. (Activate each exhaust ability only once.)",
        )
        .exhaust(),
        crew(2, "Crew 2 (Tap any number of creatures you control with total power 2 or more: This Vehicle becomes an artifact creature until end of turn.)"),
    ],
    ..oracle::RANGERS_REFUELER
};
