use cantrip_api::*;
use cantrip_mechanics::{effects, triggers::THIS_ENTERS};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(THIS_ENTERS, |g, item| effects::surveil(g, item, 1), "When this creature enters, surveil 1."),
        ability(cost("{2}{B}").and(EXILE_THIS), effects::draw_a_card_and_lose_1_life, "{2}{B}, Exile this card from your graveyard: You draw a card and you lose 1 life.")
            .from(ZoneKind::Graveyard),
    ],
    ..oracle::FAERIE_DREAMTHIEF
};
