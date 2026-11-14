use cantrip_api::*;
use cantrip_mechanics::tokens;
use cantrip_mechanics::lands;
use cantrip_mechanics::triggers::{of_subtype_you_control_dies, of_subtype_you_control_enters};
use cantrip_oracle as oracle;

/// "Whenever an Angel you control enters, you gain 4 life." (CR 603.6a): the Bishop itself counts if it is an Angel.
fn gain_life(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.gain_life(item.controller, 4);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(when_asked(EventMask::ZONE_CHANGE, of_subtype_you_control_enters::<{ lands::sub(Subtype::Angel) }>), gain_life, "Whenever an Angel you control enters, you gain 4 life."),
        triggered(when_asked(EventMask::ZONE_CHANGE, of_subtype_you_control_dies::<{ lands::sub(Subtype::Angel) }>), tokens::create_a_white_flying_spirit, "Whenever an Angel you control dies, create a 1/1 white Spirit creature token with flying."),
    ],
    ..oracle::BISHOP_OF_WINGS
};

pub static EXTRA: &[&CardDef] = &[&tokens::WHITE_FLYING_SPIRIT];
