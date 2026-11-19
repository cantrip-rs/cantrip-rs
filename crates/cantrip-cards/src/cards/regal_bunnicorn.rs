use cantrip_api::*;
use cantrip_mechanics::lands;
use cantrip_oracle as oracle;

/// "Regal Bunnicorn's power and toughness are each equal to the number of nonland permanents you control" (CR 604.3, 613.4a):
/// every permanent its controller has that is not a land, itself included, read through the layers applied so far.
fn nonland_permanents_you_control(q: &dyn LayerQuery, source: ObjectId, chars: &mut Chars) {
    let permanents = lands::count_you_control(q, source, &|c| !c.is(CardType::Land));
    chars.power = Some(permanents);
    chars.toughness = Some(permanents);
}

pub static CARD: CardDef = CardDef {
    statics: &[characteristic_defining(
        Layer::PtDefine,
        nonland_permanents_you_control,
        "Regal Bunnicorn's power and toughness are each equal to the number of nonland permanents you control.",
    )
    .lost_pt(0, 0)],
    ..oracle::REGAL_BUNNICORN
};
