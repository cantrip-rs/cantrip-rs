use cantrip_api::*;
use cantrip_mechanics::{your_creatures_have_total_power_at_least, effects, pump, targets, triggers};
use cantrip_oracle as oracle;

/// "Target attacking creature gets +2/+0 and gains first strike until end of turn." A creature that has stopped attacking
/// is no longer legal and gets neither (CR 608.2b). The two are one instruction (CR 608.2c).
fn pump_and_first_strike(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    instruction(g, |g| {
        pump(g, item.object, creature, 2, 0);
        effects::gain_until_end_of_turn(g, item.object, creature, Keyword::FirstStrike);
    });
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::YOU_ATTACK,
        pump_and_first_strike,
        "Whenever you attack while creatures you control have total power 8 or greater, target attacking creature gets +2/+0 and gains first strike until end of turn.",
    )
    .only_if(your_creatures_have_total_power_at_least::<8>)
    .targets(&[targets::ATTACKING_CREATURE])],
    ..oracle::UNA_CHIN_RILEY_NUMBER_ONE
};
