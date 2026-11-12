use cantrip_api::*;
use cantrip_mechanics::effects::{creatures_you_control_where_gain, plus_one_counter_on_target};
use cantrip_mechanics::saddle;
use cantrip_mechanics::targets;
use cantrip_mechanics::triggers::THIS_ATTACKS_WHILE_SADDLED;
use cantrip_oracle as oracle;

/// "Creatures you control with counters on them gain hexproof and indestructible until end of turn." (CR 611.2c, 702.11, 702.12):
/// the creatures with a counter of any kind as the ability resolves, the Ox already sacrificed as its cost (CR 602.2b), and no
/// creature that comes to have a counter or enters later.
fn creatures_with_counters_gain_hexproof_and_indestructible(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    creatures_you_control_where_gain(g, item, &[Keyword::Hexproof, Keyword::Indestructible], |g, id| g.state().obj(id).has_counters());
    Ok(())
}

/// "Whenever this creature attacks while saddled, put a +1/+1 counter on target creature." (CR 122.1a, 115.1): any creature, the
/// Ox's controller's or not. "Saddle 1" (CR 702.171a) is the shared mechanic.
pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            THIS_ATTACKS_WHILE_SADDLED,
            plus_one_counter_on_target,
            "Whenever this creature attacks while saddled, put a +1/+1 counter on target creature.",
        )
        .targets(&[targets::CREATURE]),
        ability(
            SACRIFICE_THIS,
            creatures_with_counters_gain_hexproof_and_indestructible,
            "Sacrifice this creature: Creatures you control with counters on them gain hexproof and indestructible until end of turn.",
        ),
        saddle::ability(1, "Saddle 1 (Tap any number of other creatures you control with total power 1 or more: This Mount becomes saddled until end of turn. Saddle only as a sorcery.)"),
    ],
    ..oracle::BULWARK_OX
};
