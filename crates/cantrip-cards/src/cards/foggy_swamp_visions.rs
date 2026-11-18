use cantrip_api::*;
use cantrip_mechanics::{delayed, targets};
use cantrip_oracle as oracle;

/// "Exile X target creature cards from graveyards. For each creature card exiled this way, create a token that's a copy of it.
/// At the beginning of your next end step, sacrifice those tokens." X is announced with the spell and its waterbend cost and
/// counts the targets (CR 601.2b-c); those still legal as it resolves are exiled together (CR 608.2b, 406.2, 101.4). The
/// tokens copy the printed cards exiled (CR 707.2), all created at once, so each sees the others enter (CR 603.6a), and that sentence is
/// over before the delayed trigger is made (CR 608.2c, 603.7c): it is about exactly those tokens.
fn exile_and_copy(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let exiled = g.exile_all(&targets::legal_objects(g, item));
    g.end_sentence();
    let copies: Vec<(Copiable, PlayerId)> = exiled.iter().map(|&card| (g.copiable_values(card), item.controller)).collect();
    let tokens = g.create_token_copies_together(&copies)?;
    delayed::at_your_next_end_step_about(g, item, 0, &tokens);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE_CARD_IN_A_GRAVEYARD.counted()], exile_and_copy),
    // "As an additional cost to cast this spell, waterbend {X}." (CR 701.67a-b, 601.2f)
    additional_cost: Some(Cost::waterbend(0, 1)),
    delayed: &[delayed_trigger(delayed::sacrifice_created, "At the beginning of your next end step, sacrifice those tokens.")],
    ..oracle::FOGGY_SWAMP_VISIONS
};
