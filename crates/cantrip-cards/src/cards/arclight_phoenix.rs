use cantrip_api::*;
use cantrip_mechanics::{history, triggers};
use cantrip_oracle as oracle;

/// "If you've cast three or more instant and sorcery spells this turn" (CR 603.4): an intervening if, asked as it
/// triggers and again as it resolves. The history keeps the spells cast this turn as they were cast (CR 601.2i).
fn cast_three_instants_and_sorceries(q: &dyn Query, asker: &Asker) -> bool {
    history::instants_and_sorceries_cast_this_turn(q, asker.controller) >= 3
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        when_asked(EventMask::STEP_BEGAN, triggers::beginning_of_your_combat),
        cantrip_mechanics::effects::return_this_card_to_battlefield,
        "At the beginning of combat on your turn, if you've cast three or more instant and sorcery spells this turn, return this card from your graveyard to the battlefield.",
    )
    .condition(cast_three_instants_and_sorceries)
    // It returns this card from the graveyard, so it functions only there (CR 113.6m).
    .functions_in(IN_A_GRAVEYARD)],
    ..oracle::ARCLIGHT_PHOENIX
};
