use cantrip_api::*;
use cantrip_mechanics::{effects, you_have_delirium};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        cost("{2}{G}{G}"),
        effects::return_this_card_with_finality,
        "Delirium \u{2014} {2}{G}{G}: Return this card from your graveyard to the battlefield with a finality counter on it. Activate only if there are four or more card types among cards in your graveyard and only as a sorcery.",
    )
    .from(ZoneKind::Graveyard)
    .sorcery_speed()
    .condition(you_have_delirium)],
    ..oracle::BALUSTRADE_WURM
};
