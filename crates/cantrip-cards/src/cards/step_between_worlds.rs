use cantrip_api::*;
use cantrip_mechanics::each_player_may_shuffle_hand_and_graveyard_in_and_draw_seven;
use cantrip_mechanics::plot::plot;
use cantrip_oracle as oracle;

/// "Each player may shuffle their hand and graveyard into their library. Each player who does draws seven cards. Exile Step
/// Between Worlds." The spell exiles itself last (CR 608.2m), so it is on the stack, not in a graveyard, while the players shuffle.
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    each_player_may_shuffle_hand_and_graveyard_in_and_draw_seven(g, item.object)?;
    g.end_sentence();
    g.exile(item.object);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[], effect),
    abilities: &[plot(
        mana("{4}{U}{U}"),
        "Plot {4}{U}{U} (You may pay {4}{U}{U} and exile this card from your hand. Cast it as a sorcery on a later turn without paying its mana cost. Plot only as a sorcery.)",
    )],
    ..oracle::STEP_BETWEEN_WORLDS
};
