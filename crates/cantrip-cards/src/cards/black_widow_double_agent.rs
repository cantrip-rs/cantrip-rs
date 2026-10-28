use cantrip_api::*;
use cantrip_mechanics::{effects, triggers};
use cantrip_oracle as oracle;

/// "It gains first strike and menace until end of turn" (CR 702.7, 702.110, 611.2a): the creature that attacked alone, the
/// trigger's subject, whichever creature of its controller's it is.
fn gain_first_strike_and_menace(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Target::Object(creature) = item.subject {
        // "It gains first strike and menace": one instruction (CR 608.2c).
        instruction(g, |g| {
            effects::gain_until_end_of_turn(g, item.object, creature, Keyword::FirstStrike);
            effects::gain_until_end_of_turn(g, item.object, creature, Keyword::Menace);
        });
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::CREATURE_YOU_CONTROL_ATTACKS_ALONE,
        gain_first_strike_and_menace,
        "Whenever a creature you control attacks alone, it gains first strike and menace until end of turn.",
    )],
    ..oracle::BLACK_WIDOW_DOUBLE_AGENT
};
