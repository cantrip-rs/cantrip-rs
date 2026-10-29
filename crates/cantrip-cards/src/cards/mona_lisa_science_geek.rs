use cantrip_api::*;
use cantrip_oracle as oracle;

/// "Add X mana of any one color, where X is Mona Lisa's power." (CR 106.5): the power as the ability resolves, at once (CR
/// 605.3b); none for a power of 0 or less. The color is always named, the ability offering all five.
fn power_in_mana(g: &mut dyn Game, asker: &Asker, color: Option<Color>) -> Result<(), Suspend> {
    let power = g.chars(asker.source).power.unwrap_or(0).max(0) as u32;
    g.add_mana(asker.controller, color.unwrap_or(Color::White).mana(), power);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[mana_ability(TAP, add_by(power_in_mana).choose_color(), "{T}: Add X mana of any one color, where X is Mona Lisa's power.")],
    ..oracle::MONA_LISA_SCIENCE_GEEK
};
