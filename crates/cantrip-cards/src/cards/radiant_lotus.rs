use cantrip_api::*;
use cantrip_mechanics::targets::{self, filters};
use cantrip_mechanics::choose_color;
use cantrip_oracle as oracle;

/// "Choose a color. Target player adds three mana of the chosen color for each artifact sacrificed this way." (CR 106.1b,
/// 105.1, 608.2): the color is chosen by the ability's controller as it resolves, the mana added to the target player, three
/// for each artifact sacrificed - the X announced with the activation (CR 107.3a), never 0 here. An illegal target gets none.
fn choose_a_color_and_add_mana(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let color = choose_color(g, item.controller, item.object)?;
    g.end_sentence();
    let Some(player) = item.target(0).player() else { return Ok(()) };
    g.add_mana(player, color.mana(), 3 * item.x);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        TAP.and(Cost { variable: Some(VariableCost::SacrificeOneOrMore(filters::artifact)), ..Cost::NONE }),
        choose_a_color_and_add_mana,
        "{T}, Sacrifice one or more artifacts: Choose a color. Target player adds three mana of the chosen color for each artifact sacrificed this way. (Activate only as an instant.)",
    )
    .targets(&[targets::PLAYER])],
    ..oracle::RADIANT_LOTUS
};
