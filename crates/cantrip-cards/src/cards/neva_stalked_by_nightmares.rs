use cantrip_api::*;
use cantrip_mechanics::{effects, targets, triggers};
use cantrip_oracle as oracle;

/// "Put a +1/+1 counter on [Neva], then scry 1." (CR 122.1, 701.22, 608.2c): the counter goes on only while Neva is still the
/// permanent that triggered (CR 400.7), the scry is done whatever became of it.
fn counter_then_scry(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::plus_one_counter_on_this(g, item)?;
    g.end_sentence();
    effects::scry(g, item, 1)
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(triggers::THIS_ENTERS, effects::return_first_target_to_hand, "When this creature enters, return target creature or enchantment card from your graveyard to your hand.")
            .targets(&[targets::CREATURE_OR_ENCHANTMENT_CARD_IN_YOUR_GRAVEYARD]),
        triggered(
            when_asked(EventMask::ZONE_CHANGE, triggers::type_you_control_to_graveyard::<{ CardType::Enchantment as u8 }>),
            counter_then_scry,
            "Whenever an enchantment you control is put into a graveyard from the battlefield, put a +1/+1 counter on this creature, then scry 1.",
        ),
    ],
    ..oracle::NEVA_STALKED_BY_NIGHTMARES
};
