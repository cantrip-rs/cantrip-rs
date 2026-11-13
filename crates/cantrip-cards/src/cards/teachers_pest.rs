use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;

/// Menace is in the characteristics. The return works only from the graveyard (CR 113.6m), where the card is not a creature.
pub static CARD: CardDef = CardDef {
    abilities: &[
        effects::attacks_gain_life::<1>("Whenever this creature attacks, you gain 1 life."),
        ability(cost("{B}{G}"), effects::return_this_card_to_battlefield_tapped, "{B}{G}: Return this card from your graveyard to the battlefield tapped.")
            .from(ZoneKind::Graveyard),
    ],
    ..oracle::TEACHERS_PEST
};
