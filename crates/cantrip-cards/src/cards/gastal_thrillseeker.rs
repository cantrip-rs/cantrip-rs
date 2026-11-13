use cantrip_api::*;
use cantrip_mechanics::max_speed::itself_at_max_speed;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_mechanics::{effects, statics, targets};
use cantrip_oracle as oracle;

/// "... has deathtouch and haste" (CR 702.2, 702.10, layer 6) as the static ability's `StaticDef::apply`.
fn have_deathtouch_and_haste(q: &dyn LayerQuery, source: ObjectId, chars: &mut Chars) {
    statics::have_deathtouch(q, source, chars);
    statics::have_haste(q, source, chars);
}

pub static CARD: CardDef = CardDef {
    start_your_engines: true,
    abilities: &[triggered(THIS_ENTERS, |g, item| effects::damage_target_and_gain_life(g, item, 1, 1), "When this creature enters, it deals 1 damage to target opponent and you gain 1 life.").targets(&[targets::OPPONENT])],
    statics: &[static_ability(Layer::Ability, itself_at_max_speed, have_deathtouch_and_haste, "Max speed \u{2014} This creature has deathtouch and haste.")],
    ..oracle::GASTAL_THRILLSEEKER
};
