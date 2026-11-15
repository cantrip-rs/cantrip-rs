use cantrip_api::*;
use cantrip_mechanics::{lands, tokens, triggers};
use cantrip_oracle as oracle;

/// The 1/1 blue Fish creature token it creates.
pub static EXTRA: &[&CardDef] = &[&tokens::FISH];

/// "Whenever another nontoken Bird you control enters, create a 1/1 blue Fish creature token." (CR 603.6a, 111.1): "Bird" is a
/// word of this card's text (CR 612), and a Bird token entering does not trigger it.
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        when_asked(EventMask::ZONE_CHANGE, triggers::another_nontoken_of_subtype_you_control_enters::<{ lands::sub(Subtype::Bird) }>),
        tokens::create_a_fish,
        "Whenever another nontoken Bird you control enters, create a 1/1 blue Fish creature token.",
    )],
    ..oracle::KNIGHTFISHER
};
