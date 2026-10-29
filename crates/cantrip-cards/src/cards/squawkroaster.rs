use cantrip_api::*;
use cantrip_oracle as oracle;

/// Vivid (CR 604.3, 105.2): power equal to the number of colors among permanents its controller controls (its owner, when it is
/// not on the battlefield), a characteristic-defining ability in every zone. Its toughness is the printed 4.
fn power_is_colors_among_permanents(q: &dyn LayerQuery, source: ObjectId, chars: &mut Chars) {
    chars.power = Some(cantrip_mechanics::vivid::count_in_layers(q, q.state().obj(source).controller) as i16);
}

pub static CARD: CardDef = CardDef {
    statics: &[characteristic_defining(Layer::PtDefine, power_is_colors_among_permanents, "Vivid \u{2014} Squawkroaster's power is equal to the number of colors among permanents you control.")],
    ..oracle::SQUAWKROASTER
};
