use cantrip_api::*;
use cantrip_mechanics::max_speed::max_speed_in_graveyard;
use cantrip_mechanics::statics::{have_vigilance, plus_pt};
use cantrip_mechanics::{attach, targets};
use cantrip_oracle as oracle;

/// "You may cast this card from your graveyard" (CR 601.3): the core keyword that gives the card the permission, which
/// the max speed ability grants (CR 702.178a) and which functions from the graveyard with it (CR 702.178b).
fn may_cast_from_your_graveyard(_: &dyn LayerQuery, _: ObjectId, chars: &mut Chars) {
    chars.keywords = chars.keywords.with(Keyword::CastFromYourGraveyard);
}

pub static CARD: CardDef = CardDef {
    start_your_engines: true,
    enchant: Some(targets::CREATURE_OR_VEHICLE),
    statics: &[
        static_ability(Layer::Ability, attach::attached_object, have_vigilance, "Enchanted permanent gets +1/+1 and has vigilance."),
        static_ability(Layer::PtModify, attach::attached_object, plus_pt::<1, 1>, "Enchanted permanent gets +1/+1 and has vigilance.").continues(),
        max_speed_in_graveyard(may_cast_from_your_graveyard, "Max speed \u{2014} You may cast this card from your graveyard."),
    ],
    ..oracle::LIGHTWHEEL_ENHANCEMENTS
};
