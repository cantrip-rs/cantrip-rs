use cantrip_api::*;
use cantrip_mechanics::effects::draw;
use cantrip_mechanics::lands;
use cantrip_mechanics::triggers::of_subtype_you_control_enters;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        when_asked(EventMask::ZONE_CHANGE, of_subtype_you_control_enters::<{ lands::sub(Subtype::Aura) }>),
        |g, item| draw(g, item, 1),
        "Whenever an Aura you control enters, draw a card.",
    )],
    ..oracle::TANGLESPAN_LOOKOUT
};
