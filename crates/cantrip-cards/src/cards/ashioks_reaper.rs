use cantrip_api::*;
use cantrip_mechanics::{effects, triggers};
use cantrip_oracle as oracle;

/// "Whenever an enchantment you control is put into a graveyard from the battlefield, draw a card." (CR 603.10a): the
/// enchantment as it last was on the battlefield, so an Aura that falls off with its creature (CR 704.5m) and an
/// enchantment token such as a Role (CR 111.7: it is put into the graveyard before it ceases to exist) count, and so does
/// an enchantment that dies together with this creature. Each enchantment is its own event, so several trigger it once each.
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        when_asked(EventMask::ZONE_CHANGE, triggers::type_you_control_to_graveyard::<{ CardType::Enchantment as u8 }>),
        |g, item| effects::draw(g, item, 1),
        "Whenever an enchantment you control is put into a graveyard from the battlefield, draw a card.",
    )],
    ..oracle::ASHIOKS_REAPER
};
