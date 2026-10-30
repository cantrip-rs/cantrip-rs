use cantrip_api::*;
use cantrip_oracle as oracle;

/// The colors among the permanents `player` controls (CR 105.2), each as the layers before this one made it: `vivid::count` for a
/// characteristic-defining ability, which is asked in the layers (`LayerQuery`) and not of a finished `Query`.
fn colors_in_layers(q: &dyn LayerQuery, player: PlayerId) -> i16 {
    let state = q.state();
    let colors = state.battlefield().iter().filter(|&&id| state.obj(id).controller == player).fold(0, |set, &id| set | q.chars(id).colors.0);
    colors.count_ones() as i16
}

/// Vivid (CR 604.3, 105.2): power equal to the number of colors among permanents its controller controls (its owner, when it is
/// not on the battlefield), a characteristic-defining ability in every zone. Its toughness is the printed 4.
fn power_is_colors_among_permanents(q: &dyn LayerQuery, source: ObjectId, chars: &mut Chars) {
    chars.power = Some(colors_in_layers(q, q.state().obj(source).controller));
}

pub static CARD: CardDef = CardDef {
    statics: &[characteristic_defining(Layer::PtDefine, power_is_colors_among_permanents, "Vivid \u{2014} Squawkroaster's power is equal to the number of colors among permanents you control.")],
    ..oracle::SQUAWKROASTER
};
