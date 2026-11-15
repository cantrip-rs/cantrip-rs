use cantrip_api::*;
use cantrip_mechanics::{damage_target, delirium, library, targets};
use cantrip_oracle as oracle;

/// "Impossible Inferno deals 6 damage to target creature. Delirium - If there are four or more card types among cards in your
/// graveyard, exile the top card of your library. You may play it until the end of your next turn" (CR 207.2c, 608.2): the
/// graveyard is counted after the damage, as the spell still resolves (a creature it kills is put there only afterwards, CR 704.3).
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    damage_target(g, item, 0, 6);
    if delirium(g, item.controller) {
        library::exile_top_card_play_until_end_of_next_turn(g, item)?;
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE], effect),
    ..oracle::IMPOSSIBLE_INFERNO
};
