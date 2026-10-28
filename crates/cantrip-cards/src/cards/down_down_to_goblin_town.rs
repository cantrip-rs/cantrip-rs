use cantrip_api::*;
use cantrip_mechanics::amass::amass_goblins;
use cantrip_mechanics::effects::target_player_loses_1_life_you_gain_1;
use cantrip_mechanics::hand::discard_chosen_nonland_from_revealed_hand;
use cantrip_mechanics::{targets, tokens};
use cantrip_oracle as oracle;

pub static EXTRA: &[&CardDef] = &[&tokens::GOBLIN_ARMY];

/// I - "Target opponent reveals their hand. You choose a nonland card from it. That player discards that card." With its
/// target illegal the chapter does not resolve at all (CR 608.2b).
fn reveal_and_discard(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(opponent) = item.target(0).player() else { return Ok(()) };
    discard_chosen_nonland_from_revealed_hand(g, item, opponent)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        chapter(1, reveal_and_discard, "I \u{2014} Target opponent reveals their hand. You choose a nonland card from it. That player discards that card.")
            .targets(&[targets::OPPONENT]),
        chapter(2, amass_goblins::<1>, "II \u{2014} Amass Goblins 1."),
        chapter(3, target_player_loses_1_life_you_gain_1, "III, IV \u{2014} Target opponent loses 1 life and you gain 1 life.")
            .targets(&[targets::OPPONENT]),
        chapter(4, target_player_loses_1_life_you_gain_1, "III, IV \u{2014} Target opponent loses 1 life and you gain 1 life.")
            .targets(&[targets::OPPONENT]),
    ],
    ..oracle::DOWN_DOWN_TO_GOBLIN_TOWN
};
