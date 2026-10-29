use cantrip_api::*;
use cantrip_mechanics::increment;
use cantrip_oracle as oracle;

/// "{T}: Add an amount of {G} equal to this creature's power" (CR 106.1, 107.1b): its power as the ability resolves, which adds
/// nothing if it is 0 or less.
fn add_its_power(g: &mut dyn Game, asker: &Asker, _: Option<Color>) -> Result<(), Suspend> {
    let power = g.chars(asker.source).power.unwrap_or(0).max(0) as u32;
    g.add_mana(asker.controller, Mana::G, power);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[increment::TRIGGER, mana_ability(TAP, add_by(add_its_power), "{T}: Add an amount of {G} equal to this creature's power.")],
    ..oracle::TOPIARY_LECTURER
};
