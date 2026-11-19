use cantrip_api::*;
use cantrip_mechanics::lands;
use cantrip_mechanics::statics::{you_control_a, have_hexproof, itself_while_untapped};
use cantrip_oracle as oracle;

/// Flying is a core keyword.
pub static CARD: CardDef = CardDef {
    flash_if: Some(you_control_a::<{ lands::sub(Subtype::Faerie) }>),
    statics: &[static_ability(Layer::Ability, itself_while_untapped, have_hexproof, "This creature has hexproof as long as it's untapped.")],
    ..oracle::ILLUSION_SPINNERS
};
