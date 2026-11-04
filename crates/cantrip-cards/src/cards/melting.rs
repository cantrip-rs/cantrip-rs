use cantrip_api::*;
use cantrip_oracle as oracle;

/// "All lands are no longer snow." (CR 604.1).
fn all_lands_no_longer_snow_affects(
    _: &dyn LayerQuery,
    _: ObjectId,
    _: ObjectId,
    so_far: &Chars,
) -> bool {
    (*so_far).is(CardType::Land)
}

/// "All lands are no longer snow." (CR 604.1).
fn all_lands_no_longer_snow_apply_0(_: &dyn LayerQuery, _: ObjectId, chars: &mut Chars) {
    chars.supertypes.snow = false;
}

pub static CARD: CardDef = CardDef {
    statics: &[
        static_ability(
            Layer::Type,
            all_lands_no_longer_snow_affects,
            all_lands_no_longer_snow_apply_0,
            "All lands are no longer snow.",
        ),
    ],
    ..oracle::MELTING
};
