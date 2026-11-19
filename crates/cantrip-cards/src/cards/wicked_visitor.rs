use cantrip_api::*;
use cantrip_mechanics::{effects, triggers};
use cantrip_oracle as oracle;

/// "Whenever an enchantment you control is put into a graveyard from the battlefield" (CR 700.4): a Role token counts; this
/// creature is not an enchantment itself, so its own death does not.
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        when_asked(EventMask::ZONE_CHANGE, triggers::type_you_control_to_graveyard::<{ CardType::Enchantment as u8 }>),
        effects::each_opponent_loses_1_life,
        "Whenever an enchantment you control is put into a graveyard from the battlefield, each opponent loses 1 life.",
    )],
    ..oracle::WICKED_VISITOR
};
