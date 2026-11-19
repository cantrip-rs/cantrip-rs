use cantrip_api::*;
use cantrip_mechanics::{tokens, triggers};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        when_asked(EventMask::ZONE_CHANGE, triggers::type_you_control_to_graveyard::<{ CardType::Enchantment as u8 }>),
        |g, item| tokens::create(g, item.controller, &tokens::BIRD, 1).map(|_| ()),
        "Whenever an enchantment you control is put into a graveyard from the battlefield, create a 1/1 white Bird creature token with flying.",
    )],
    ..oracle::KNIGHT_OF_DOVES
};

pub static EXTRA: &[&CardDef] = &[&tokens::BIRD];
