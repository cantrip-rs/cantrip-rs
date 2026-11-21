use cantrip_api::*;
use cantrip_mechanics::history;
use cantrip_mechanics::triggers;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        when_asked(EventMask::STEP_BEGAN, triggers::beginning_of_your_combat),
        cantrip_mechanics::effects::return_this_card_to_battlefield,
        "At the beginning of combat on your turn, if two or more creatures died this turn, return this card from your graveyard to the battlefield.",
    )
    .condition(history::n_or_more_creatures_died_this_turn::<2>)
    // It returns this card from the graveyard, so it functions only there (CR 113.6m).
    .functions_in(IN_A_GRAVEYARD)],
    ..oracle::DARKLIGHT_PHOENIX
};
