use cantrip_api::*;
use cantrip_mechanics::{effects, tokens, triggers};
use cantrip_oracle as oracle;

/// "Whenever an enchantment you control is put into a graveyard from the battlefield" (CR 700.4), a Role token included, "create
/// a 1/1 black Rat creature token with 'This token can't block.'" and "{1}{B}: This creature gains deathtouch until end of turn."
pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            when_asked(EventMask::ZONE_CHANGE, triggers::type_you_control_to_graveyard::<{ CardType::Enchantment as u8 }>),
            tokens::create_rats_that_cant_block::<1>,
            "Whenever an enchantment you control is put into a graveyard from the battlefield, create a 1/1 black Rat creature token with \"This token can't block.\"",
        ),
        ability(cost("{1}{B}"), effects::this_gains_deathtouch, "{1}{B}: This creature gains deathtouch until end of turn."),
    ],
    ..oracle::WAREHOUSE_TABBY
};

pub static EXTRA: &[&CardDef] = &[&tokens::RAT_CANT_BLOCK];
