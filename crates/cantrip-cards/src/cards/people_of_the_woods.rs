use cantrip_api::*;
use cantrip_mechanics::lands;
use cantrip_oracle as oracle;

/// "People of the Woods's toughness is equal to the number of Forests you control." (CR 604.1).
fn people_woodss_toughness_equal_number_apply_0(
    q: &dyn LayerQuery,
    source: ObjectId,
    chars: &mut Chars,
) {
    let subtype = q.words(source).subtype(Subtype::Forest);
    chars.toughness = Some(lands::count_you_control(q, source, &|c| c.has_subtype(subtype)));
}

pub static CARD: CardDef = CardDef {
    power: Some(1),
    toughness: Some(0),
    statics: &[
        characteristic_defining(
            Layer::PtDefine,
            people_woodss_toughness_equal_number_apply_0,
            "People of the Woods's toughness is equal to the number of Forests you control.",
        ),
    ],
    ..oracle::PEOPLE_OF_THE_WOODS
};
