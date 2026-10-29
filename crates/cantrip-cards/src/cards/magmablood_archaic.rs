use cantrip_api::*;
use cantrip_mechanics::{effects, spent, sunburst, triggers};
use cantrip_oracle as oracle;

/// "Whenever you cast an instant or sorcery spell, creatures you control get +1/+0 until end of turn for each color of mana spent
/// to cast that spell." The spell is the one the ability triggered on (`item.subject`); its mana spent is read from its own object,
/// so the count holds even if it has since been countered (CR 608.2h). The ability resolves before the spell (CR 603.3), and
/// affects the creatures there as it resolves (CR 611.2c).
fn pump_creatures(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Target::Object(spell) = item.subject {
        let colors = sunburst::colors_spent(g, spell) as i16;
        effects::pump_creatures_you_control(g, item, colors, 0);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    enters: Some(spent::enters_with_a_plus_one_counter_for_each_color_spent),
    abilities: &[triggered(
        triggers::INSTANT_OR_SORCERY_YOU_CAST,
        pump_creatures,
        "Whenever you cast an instant or sorcery spell, creatures you control get +1/+0 until end of turn for each color of mana spent to cast that spell.",
    )],
    ..oracle::MAGMABLOOD_ARCHAIC
};
