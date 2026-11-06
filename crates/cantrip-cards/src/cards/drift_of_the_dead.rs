use cantrip_api::*;
use cantrip_mechanics::lands;
use cantrip_oracle as oracle;

/// Power and toughness are each the number of snow lands its controller controls (CR 604.3).
fn power_and_toughness(q: &dyn LayerQuery, source: ObjectId, chars: &mut Chars) {
    let n = lands::count_you_control(q, source, &|c| c.supertypes.snow && c.is(CardType::Land));
    chars.power = Some(n);
    chars.toughness = Some(n);
}

pub static CARD: CardDef = CardDef {
    power: Some(0),
    toughness: Some(0),
    statics: &[characteristic_defining(
        Layer::PtDefine,
        power_and_toughness,
        "Drift of the Dead's power and toughness are each equal to the number of snow lands you control.",
    )],
    ..oracle::DRIFT_OF_THE_DEAD
};
