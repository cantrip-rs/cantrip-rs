use cantrip_api::*;
use cantrip_mechanics::{history, targets, triggers};
use cantrip_oracle as oracle;

/// Bloodline Recollector's prepare spell (CR 722): not a card, only ever a copy.
pub static ANCESTRAL_CRAVING: CardDef = CardDef {
    name: "Ancestral Craving",
    cost: mana("{B}"),
    types: TypeSet::of(&[CardType::Instant]),
    spell: spell(&[targets::PLAYER], draw_three_lose_three),
    text: "Target player draws three cards and loses 3 life.",
    ..CardDef::BLANK
};

/// "Target player draws three cards and loses 3 life." (CR 121.1, 118.3.)
fn draw_three_lose_three(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Some(player) = targets::player(g, item, 0) {
        g.draw(player, 3);
        g.lose_life(player, 3);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(triggers::EACH_END_STEP, cantrip_mechanics::prepare::become_prepared, "At the beginning of each end step, if three or more creatures died this turn, this creature becomes prepared.")
        .condition(history::n_or_more_creatures_died_this_turn::<3>)],
    prepare: Some(&ANCESTRAL_CRAVING),
    ..oracle::BLOODLINE_RECOLLECTOR
};

pub static EXTRA: &[&CardDef] = &[&ANCESTRAL_CRAVING];
