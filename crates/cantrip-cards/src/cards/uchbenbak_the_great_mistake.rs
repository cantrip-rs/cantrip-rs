use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        cost("{4}{U}{B}"),
        effects::return_this_card_with_finality,
        "Descend 8 \u{2014} {4}{U}{B}: Return this card from your graveyard to the battlefield with a finality counter on it. Activate only if there are eight or more permanent cards in your graveyard and only as a sorcery.",
    )
    .from(ZoneKind::Graveyard)
    .sorcery_speed()
    .condition(targets::you_have_permanent_cards_in_graveyard::<8>)],
    ..oracle::UCHBENBAK_THE_GREAT_MISTAKE
};
