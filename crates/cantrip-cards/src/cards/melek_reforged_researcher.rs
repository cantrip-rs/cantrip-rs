use cantrip_api::*;
use cantrip_mechanics::{cost_reduction, statics};
use cantrip_oracle as oracle;

/// "Melek's power and toughness are each equal to twice the number of instant and sorcery cards in your graveyard" (CR 604.3, layer
/// 7a): a characteristic-defining ability, counted whenever characteristics are computed. Without it Melek is 0/0 (CR 208.5).
fn power_and_toughness_are_twice_instants_and_sorceries_in_your_graveyard(q: &dyn LayerQuery, source: ObjectId, chars: &mut Chars) {
    let twice = 2 * statics::instant_and_sorcery_cards_in_your_graveyard(q, source) as i32;
    chars.power = Some(twice);
    chars.toughness = Some(twice);
}

fn instant_or_sorcery(chars: &Chars) -> bool {
    chars.is(CardType::Instant) || chars.is(CardType::Sorcery)
}

/// "The first instant or sorcery spell you cast each turn costs {3} less to cast" (CR 601.2f, 118.7): generic mana only, and an
/// instant or sorcery spell cast earlier this turn, before Melek was on the battlefield or not, makes this one not the first.
fn first_instant_or_sorcery_costs_three_less(q: &dyn Query, source: ObjectId, player: PlayerId, costed: Costed, _: &[Target]) -> CostDelta {
    cost_reduction::first_spell_you_cast_each_turn(q, source, player, costed, 3, instant_or_sorcery)
}

pub static CARD: CardDef = CardDef {
    statics: &[characteristic_defining(
        Layer::PtDefine,
        power_and_toughness_are_twice_instants_and_sorceries_in_your_graveyard,
        "Melek's power and toughness are each equal to twice the number of instant and sorcery cards in your graveyard.",
    )
    .lost_pt(0, 0)],
    cost_changes: &[first_instant_or_sorcery_costs_three_less],
    ..oracle::MELEK_REFORGED_RESEARCHER
};
