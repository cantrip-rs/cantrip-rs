use cantrip_api::*;
use cantrip_mechanics::{mana, statics};
use cantrip_oracle as oracle;

/// "... has first strike and deathtouch" (CR 702.7, 702.2, layer 6) as the static ability's `StaticDef::apply`.
fn have_first_strike_and_deathtouch(q: &dyn LayerQuery, source: ObjectId, chars: &mut Chars) {
    statics::have_first_strike(q, source, chars);
    statics::have_deathtouch(q, source, chars);
}

pub static CARD: CardDef = CardDef {
    enters: Some(mana::enters_tapped_with_stun_counters::<1>),
    statics: &[static_ability(
        Layer::Ability,
        statics::itself_during_your_turn,
        have_first_strike_and_deathtouch,
        "Chef's Knife \u{2014} During your turn, this creature has first strike and deathtouch.",
    )],
    ..oracle::TONBERRY
};
