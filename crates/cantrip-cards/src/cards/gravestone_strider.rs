use cantrip_api::*;
use cantrip_mechanics::{effects, mana, targets};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        mana::ONE_FOR_ANY_COLOR_ONCE_EACH_TURN,
        // It functions only in its owner's graveyard (CR 113.6m). Its target is chosen before the cost exiles it (CR 602.2b, 601.2c),
        // so it may target itself, and then does nothing (CR 608.2b).
        ability(cost("{2}").and(EXILE_THIS), effects::exile_first_target, "{2}, Exile this card from your graveyard: Exile target card from a graveyard.")
            .targets(&[targets::CARD_IN_A_GRAVEYARD])
            .from(ZoneKind::Graveyard),
    ],
    ..oracle::GRAVESTONE_STRIDER
};
