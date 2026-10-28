use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_mechanics::max_speed::itself_at_max_speed;
use cantrip_oracle as oracle;

/// "Max speed \u{2014} This creature has double strike." (CR 702.178a, 702.4, layer 6): while its controller's speed is 4.
fn have_double_strike(_: &dyn LayerQuery, _: ObjectId, chars: &mut Chars) {
    chars.keywords = chars.keywords.with(Keyword::DoubleStrike);
}

pub static CARD: CardDef = CardDef {
    start_your_engines: true,
    abilities: &[ability(cost("{2}"), effects::this_gets::<1, 0>, "{2}: This creature gets +1/+0 until end of turn.")],
    statics: &[static_ability(Layer::Ability, itself_at_max_speed, have_double_strike, "Max speed \u{2014} This creature has double strike.")],
    ..oracle::BURNOUT_BASHTRONAUT
};
