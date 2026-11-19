use cantrip_api::*;
use cantrip_mechanics::{effects, graveyards, targets};
use cantrip_oracle as oracle;

/// "Cruel Somnophage's power and toughness are each equal to the number of creature cards in all graveyards." (CR 604.3).
fn power_and_toughness_are_creature_cards_in_all_graveyards(
    q: &dyn LayerQuery,
    _: ObjectId,
    chars: &mut Chars,
) {
    let count = graveyards::creature_cards_in_graveyards_in_layers(q).count() as i32;
    chars.power = Some(count);
    chars.toughness = Some(count);
}

pub static CANT_WAKE_UP: CardDef = CardDef {
    spell: spell(&[targets::PLAYER], effects::target_player_mills::<4>),
    ..oracle::CANT_WAKE_UP
};

pub static CARD: CardDef = CardDef {
    statics: &[characteristic_defining(
        Layer::PtDefine,
        power_and_toughness_are_creature_cards_in_all_graveyards,
        "Cruel Somnophage's power and toughness are each equal to the number of creature cards in all graveyards.",
    )
    .lost_pt(0, 0)],
    adventure: Some(&CANT_WAKE_UP),
    ..oracle::CRUEL_SOMNOPHAGE
};

pub static EXTRA: &[&CardDef] = &[&CANT_WAKE_UP];
