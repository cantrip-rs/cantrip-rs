use cantrip_api::*;
use cantrip_mechanics::history;
use cantrip_mechanics::plot::plot;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    enters: Some(history::enters_with_a_plus_one_counter_if_a_creature_died),
    abilities: &[plot(
        mana("{1}{B}"),
        "Plot {1}{B} (You may pay {1}{B} and exile this card from your hand. Cast it as a sorcery on a later turn without paying its mana cost. Plot only as a sorcery.)",
    )],
    ..oracle::BLACKSNAG_BUZZARD
};
