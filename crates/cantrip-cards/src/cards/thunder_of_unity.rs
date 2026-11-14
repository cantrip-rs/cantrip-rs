use cantrip_api::*;
use cantrip_oracle as oracle;

/// II, III: the delayed ability that lasts the turn (CR 603.7b), made by the chapter. "You" is the Saga's controller as
/// the chapter resolved (CR 603.7d).
fn watch(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    cantrip_mechanics::delayed::this_turn(g, item, item.object);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        chapter(1, cantrip_mechanics::effects::draw_two_cards_and_lose_2_life, "I \u{2014} You draw two cards and you lose 2 life."),
        chapter(2, watch, "II \u{2014} Whenever a creature you control enters this turn, each opponent loses 1 life and you gain 1 life."),
        chapter(3, watch, "III \u{2014} Whenever a creature you control enters this turn, each opponent loses 1 life and you gain 1 life."),
    ],
    delayed: &[triggered(cantrip_mechanics::triggers::CREATURE_YOU_CONTROL_ENTERS, cantrip_mechanics::effects::each_opponent_loses_1_life_you_gain_1, "Whenever a creature you control enters this turn, each opponent loses 1 life and you gain 1 life.")],
    ..oracle::THUNDER_OF_UNITY
};
