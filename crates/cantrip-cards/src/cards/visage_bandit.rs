use cantrip_api::*;
use cantrip_mechanics::copy::enter_as_copy_of_creature_you_control_and_a_shapeshifter_rogue;
use cantrip_mechanics::plot::plot;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    enters_as_copy: Some(enter_as_copy_of_creature_you_control_and_a_shapeshifter_rogue),
    abilities: &[plot(
        mana("{2}{U}"),
        "Plot {2}{U} (You may pay {2}{U} and exile this card from your hand. Cast it as a sorcery on a later turn without paying its mana cost. Plot only as a sorcery.)",
    )],
    ..oracle::VISAGE_BANDIT
};
