use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "Another target creature card from your graveyard" (CR 115.1, 601.2c): a creature card of your graveyard other than this card,
/// which is still there as the ability is activated (its exile is a cost, paid after the targets are chosen, CR 601.2h).
fn another_creature_card_in_your_graveyard(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    targets::card_in_your_graveyard(q, asker, t, &[CardType::Creature]) && t != Target::Object(asker.source)
}

pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        cost("{3}{B}").and(EXILE_THIS),
        effects::return_first_target_to_hand,
        "{3}{B}, Exile this card from your graveyard: Return another target creature card from your graveyard to your hand.",
    )
    .targets(&[target(another_creature_card_in_your_graveyard)])
    .from(ZoneKind::Graveyard)],
    ..oracle::BONEBIND_ORATOR
};
