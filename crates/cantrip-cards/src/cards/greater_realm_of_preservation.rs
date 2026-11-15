use cantrip_api::*;
use cantrip_mechanics::prevention;
use cantrip_oracle as oracle;

const BLACK_OR_RED: u8 = ColorSet::of(Color::Black).0 | ColorSet::of(Color::Red).0;

/// "{1}{W}: The next time a black or red source of your choice would deal damage to you this turn, prevent that damage." (CR 609.7a, 615.8).
fn next_time_black_red_source(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    prevention::next_time_source_of_colors_would_damage_you::<BLACK_OR_RED>(g, item)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(
            cost("{1}{W}"),
            next_time_black_red_source,
            "{1}{W}: The next time a black or red source of your choice would deal damage to you this turn, prevent that damage.",
        ),
    ],
    granted_preventions: &[
        GrantedPreventionDef {
            hint: "{1}{W}: The next time a black or red source of your choice would deal damage to you this turn, prevent that damage.",
            damage: "damage",
            of_it: None,
            applies: prevention::from_chosen_source_of_colors::<BLACK_OR_RED>,
            portion: None,
            then: None,
            gain_prevented: false,
            text: "The next time a black or red source of your choice would deal damage to you this turn, prevent that damage.",
        },
    ],
    ..oracle::GREATER_REALM_OF_PRESERVATION
};
