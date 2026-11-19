use cantrip_api::*;
use cantrip_mechanics::{effects, triggers};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        when_asked(EventMask::ZONE_CHANGE, triggers::type_you_control_to_graveyard::<{ CardType::Enchantment as u8 }>),
        effects::plus_one_counter_on_this,
        "Whenever an enchantment you control is put into a graveyard from the battlefield, put a +1/+1 counter on this creature.",
    )],
    ..oracle::SAVIOR_OF_THE_SLEEPING
};
