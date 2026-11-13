use cantrip_api::*;
use cantrip_mechanics::crew::crew;
use cantrip_mechanics::{effects, tokens, triggers};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(triggers::THIS_ATTACKS, tokens::create_a_treasure, "Whenever this Vehicle attacks, create a Treasure token."),
        ability(
            cost("{3}"),
            effects::this_vehicle_becomes_artifact_creature_with_plus_one_counters::<1>,
            "Exhaust \u{2014} {3}: This Vehicle becomes an artifact creature. Put a +1/+1 counter on it. (Activate each exhaust ability only once.)",
        )
        .exhaust(),
        crew(1, "Crew 1 (Tap any number of creatures you control with total power 1 or more: This Vehicle becomes an artifact creature until end of turn.)"),
    ],
    ..oracle::ROCKETEER_BOOSTBUGGY
};

pub static EXTRA: &[&CardDef] = &[&tokens::TREASURE];
