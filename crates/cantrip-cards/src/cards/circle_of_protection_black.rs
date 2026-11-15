use cantrip_api::*;
use cantrip_mechanics::prevention;
use cantrip_oracle as oracle;

const BLACK: u8 = ColorSet::of(Color::Black).0;

/// "{1}: The next time a black source of your choice would deal damage to you this turn, prevent that damage." (CR 609.7a, 615.8).
fn next_time_black_source_choice(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    prevention::next_time_source_of_colors_would_damage_you::<BLACK>(g, item)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(
            cost("{1}"),
            next_time_black_source_choice,
            "{1}: The next time a black source of your choice would deal damage to you this turn, prevent that damage.",
        ),
    ],
    granted_preventions: &[
        GrantedPreventionDef {
            hint: "{1}: The next time a black source of your choice would deal damage to you this turn, prevent that damage.",
            damage: "damage",
            of_it: None,
            applies: prevention::from_chosen_source_of_colors::<BLACK>,
            portion: None,
            then: None,
            gain_prevented: false,
            text: "The next time a black source of your choice would deal damage to you this turn, prevent that damage.",
        },
    ],
    ..oracle::CIRCLE_OF_PROTECTION_BLACK
};
