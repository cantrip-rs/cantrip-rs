use cantrip_api::*;
use cantrip_mechanics::{delirium, effects};
use cantrip_oracle as oracle;

/// "Activate only if there are four or more card types among cards in your graveyard" (CR 602.5, 207.2c).
fn has_delirium(q: &dyn Query, asker: &Asker) -> bool {
    delirium(q, asker.controller)
}

pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        cost("{2}{B}{B}"),
        effects::return_this_card_with_finality,
        "Delirium \u{2014} {2}{B}{B}: Return this card from your graveyard to the battlefield with a finality counter on it. Activate only if there are four or more card types among cards in your graveyard and only as a sorcery.",
    )
    .sorcery_speed()
    .condition(has_delirium)
    .from(ZoneKind::Graveyard)],
    ..oracle::RESURRECTED_CULTIST
};
