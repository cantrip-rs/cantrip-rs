use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// The legal target gains first strike until end of turn (CR 702.7a, 611.2a).
fn first_strike(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    effects::gain_until_end_of_turn(g, item.object, creature, Keyword::FirstStrike);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[ability(cost("{1}{W}"), first_strike, "{1}{W}: Target green creature gains first strike until end of turn.")
        .targets(&[targets::creature_of_color::<{ Color::Green as u8 }>()])],
    ..oracle::KELSINKO_RANGER
};
