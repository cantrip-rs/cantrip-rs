use cantrip_api::*;
use cantrip_mechanics::{card_types, double, effects, triggers, untapped_you_control, your_creatures_have_total_power_at_least};
use cantrip_oracle as oracle;

/// "Double Worf's power until end of turn" (CR 701.10a-b): +X/+0, where X is his power as the ability resolves. A Worf who
/// has left the battlefield is gone (CR 400.7), so nothing happens.
fn double_power(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    double::power(g, item.object, item.object);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            triggers::YOU_ATTACK,
            double_power,
            "Whenever you attack while creatures you control have total power 8 or greater, double Worf's power until end of turn.",
        )
        .only_if(your_creatures_have_total_power_at_least::<8>),
        ability(
            Cost::tap_untapped(2, untapped_you_control::<{ card_types(&[CardType::Artifact, CardType::Creature]) }>),
            effects::plus_one_counter_on_this,
            "Tap two untapped artifacts and/or creatures you control: Put a +1/+1 counter on Worf. Activate only as a sorcery.",
        )
        .sorcery_speed(),
    ],
    ..oracle::WORF_HONOR_BOUND_WARRIOR
};
