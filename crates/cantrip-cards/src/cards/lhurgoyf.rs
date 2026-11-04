use cantrip_api::*;
use cantrip_mechanics::graveyards;
use cantrip_oracle as oracle;

/// "Lhurgoyf's power is equal to the number of creature cards in all graveyards and its toughness is equal to that number plus 1." (CR 604.3).
fn lhurgoyfs_power_equal_number_creature_apply_0(
    q: &dyn LayerQuery,
    _: ObjectId,
    chars: &mut Chars,
) {
    let count = graveyards::cards_in_graveyards(q.state())
        .filter(|&card| q.chars(card).is(CardType::Creature))
        .count();
    chars.power = Some(count as i32);
    chars.toughness = Some((count + 1) as i32);
}

pub static CARD: CardDef = CardDef {
    statics: &[
        characteristic_defining(
            Layer::PtDefine,
            lhurgoyfs_power_equal_number_creature_apply_0,
            "Lhurgoyf's power is equal to the number of creature cards in all graveyards and its toughness is equal to that number plus 1.",
        ).lost_pt(0, 1),
    ],
    ..oracle::LHURGOYF
};
