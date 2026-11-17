use cantrip_api::*;
use cantrip_mechanics::mana::one_of_the_chosen_color_per_different_power;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[mana_ability(
        TAP,
        add_by(one_of_the_chosen_color_per_different_power).choose_color(),
        "{T}: Choose a color. Add one mana of that color for each different power among creatures you control.",
    )],
    ..oracle::LOOT_THE_NEXUS
};
