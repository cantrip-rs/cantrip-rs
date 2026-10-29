use cantrip_api::*;
use cantrip_mechanics::lands;
use cantrip_mechanics::statics::{flash_if_you_control_a, have_hexproof};
use cantrip_oracle as oracle;

/// "This creature has hexproof as long as it's untapped" (CR 702.11, 604.1, 611.3a): only the creature itself, and only while
/// it is untapped, asked again whenever characteristics are computed, so the hexproof stops the moment it taps (attacking
/// included, as it has no vigilance) and returns as it untaps.
fn itself_while_untapped(view: &dyn LayerQuery, source: ObjectId, affected: ObjectId, _: &Chars) -> bool {
    source == affected && !view.state().obj(source).tapped
}

/// Flying is a core keyword.
pub static CARD: CardDef = CardDef {
    flash_if: Some(flash_if_you_control_a::<{ lands::sub(Subtype::Faerie) }>),
    statics: &[static_ability(Layer::Ability, itself_while_untapped, have_hexproof, "This creature has hexproof as long as it's untapped.")],
    ..oracle::ILLUSION_SPINNERS
};
