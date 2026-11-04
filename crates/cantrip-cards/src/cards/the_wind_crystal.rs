use cantrip_api::*;
use cantrip_mechanics::replacements::GAIN_TWICE_THAT_MUCH_LIFE;
use cantrip_mechanics::{cost_reduction, effects};
use cantrip_oracle as oracle;

/// "Creatures you control gain flying and lifelink until end of turn." (CR 611.2c)
fn take_wing(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::creatures_you_control_gain(g, item, &[Keyword::Flying, Keyword::Lifelink]);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    cost_changes: &[cost_reduction::spells_of_color_you_cast_cost_one_less::<{ Color::White as u8 }>],
    replacements: &[GAIN_TWICE_THAT_MUCH_LIFE],
    abilities: &[ability(cost("{4}{W}{W}, {T}"), take_wing, "{4}{W}{W}, {T}: Creatures you control gain flying and lifelink until end of turn.")],
    ..oracle::THE_WIND_CRYSTAL
};
