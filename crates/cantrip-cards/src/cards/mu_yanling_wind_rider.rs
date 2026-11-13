use cantrip_api::*;
use cantrip_mechanics::statics::{have_flying, your_of_subtype};
use cantrip_mechanics::{effects, lands, tokens, triggers};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            triggers::THIS_ENTERS,
            tokens::create_a_crew_1_vehicle,
            "When Mu Yanling enters, create a 3/2 colorless Vehicle artifact token with crew 1.",
        ),
        triggered(
            when_asked(EventMask::DAMAGE, triggers::creature_you_control_with_flying_deals_combat_damage_to_player),
            |g, item| effects::draw(g, item, 1),
            "Whenever one or more creatures you control with flying deal combat damage to a player, draw a card.",
        )
        .sums_per_subject(),
    ],
    statics: &[static_ability(
        Layer::Ability,
        your_of_subtype::<{ lands::sub(Subtype::Vehicle) }>,
        have_flying,
        "Vehicles you control have flying.",
    )],
    ..oracle::MU_YANLING_WIND_RIDER
};

pub static EXTRA: &[&CardDef] = &[&tokens::VEHICLE_CREW_1];
