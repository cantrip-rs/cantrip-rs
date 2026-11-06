use cantrip_api::*;
use cantrip_mechanics::statics;
use cantrip_oracle as oracle;

/// The Mammoths have trample while their controller controls a snow land (CR 604.1).
fn has_snow_land(q: &dyn LayerQuery, source: ObjectId, affected: ObjectId, _: &Chars) -> bool {
    source == affected && q.state().battlefield().iter().copied().any(|id| {
        if q.controller(id) != q.controller(source) {
            return false;
        }
        let chars = q.chars(id);
        chars.supertypes.snow && chars.is(CardType::Land)
    })
}

pub static CARD: CardDef = CardDef {
    statics: &[static_ability(
        Layer::Ability,
        has_snow_land,
        statics::have_trample,
        "This creature has trample as long as you control a snow land.",
    )],
    ..oracle::WOOLLY_MAMMOTHS
};
