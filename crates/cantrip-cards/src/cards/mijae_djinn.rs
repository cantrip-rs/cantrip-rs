use cantrip_api::*;
use cantrip_mechanics::triggers;
use cantrip_oracle as oracle;

/// "Whenever this creature attacks, flip a coin. If you lose the flip, remove this creature from combat and tap it." (CR 608.2, 701.26a).
fn flip_coin_if_lose_flip(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let won = g.flip_coin(item.controller);
    g.end_sentence();
    if !won && is_live(g, item.object) {
        instruction(g, |g| {
            g.remove_from_combat(item.object);
            g.tap_together(&[item.object]);
        });
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            triggers::THIS_ATTACKS,
            flip_coin_if_lose_flip,
            "Whenever this creature attacks, flip a coin. If you lose the flip, remove this creature from combat and tap it.",
        ),
    ],
    ..oracle::MIJAE_DJINN
};
