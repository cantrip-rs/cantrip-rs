use cantrip_api::*;
use cantrip_mechanics::triggers::YOUR_FIRST_MAIN_PHASE;
use cantrip_oracle as oracle;

/// "At the beginning of your first main phase, add {U}{R}. Spend this mana only to cast instant and sorcery spells." (CR 106.4,
/// 106.6): each mana is restricted separately, and empties from the pool with the step or phase (CR 106.4).
fn add_blue_red(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.add_restricted_mana(item.controller, Mana::U, 1, ManaRestriction::INSTANT_OR_SORCERY_SPELLS);
    g.add_restricted_mana(item.controller, Mana::R, 1, ManaRestriction::INSTANT_OR_SORCERY_SPELLS);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        YOUR_FIRST_MAIN_PHASE,
        add_blue_red,
        "At the beginning of your first main phase, add {U}{R}. Spend this mana only to cast instant and sorcery spells.",
    )],
    ..oracle::ABSTRACT_PAINTMAGE
};
