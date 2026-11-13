use cantrip_api::*;
use cantrip_mechanics::statics::{have_haste, your_of_subtype};
use cantrip_mechanics::triggers::YOU_ATTACK;
use cantrip_mechanics::{effects, history, lands};
use cantrip_oracle as oracle;

/// "If a Pirate and a Vehicle attacked this combat" (CR 603.4, 508.1): among the creatures declared as attackers in this combat phase, one is
/// a Pirate and one is a Vehicle. One creature that is both is a Pirate and a Vehicle (the rulings). The types are words of this card's text
/// (CR 612). As the ability triggers the declaration is still being made - its `Attacks` records are not in the history yet - so the
/// declared attackers (`State::attackers`) are the ones that attacked; as it resolves, the history has them as they were declared (those
/// that have left since still count, and one put onto the battlefield attacking was never declared).
fn a_pirate_and_a_vehicle_attacked(q: &dyn Query, asker: &Asker) -> bool {
    let words = q.words(asker.source);
    let (pirate, vehicle) = (words.subtype(Subtype::Pirate), words.subtype(Subtype::Vehicle));
    let mut attacked: Vec<_> = history::this_combat(q)
        .iter()
        .filter_map(|r| match r.event {
            Event::Attacks { attacker, known, .. } => Some(Attacked { attacker, known }.chars(q)),
            _ => None,
        })
        .collect();
    if attacked.is_empty() {
        attacked = q.state().attackers.iter().map(|a| q.chars(a.id)).collect();
    }
    attacked.iter().any(|c| c.has_subtype(pirate)) && attacked.iter().any(|c| c.has_subtype(vehicle))
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        YOU_ATTACK,
        effects::draw_three_then_discard_two,
        "Whenever you attack, if a Pirate and a Vehicle attacked this combat, draw three cards, then discard two cards.",
    )
    .condition(a_pirate_and_a_vehicle_attacked)],
    statics: &[static_ability(Layer::Ability, your_of_subtype::<{ lands::sub(Subtype::Vehicle) }>, have_haste, "Vehicles you control have haste.")],
    ..oracle::FEARLESS_SWASHBUCKLER
};
