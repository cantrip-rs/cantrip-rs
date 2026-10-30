use cantrip_api::*;
use cantrip_mechanics::{delayed, targets};
use cantrip_oracle as oracle;

/// "For each of them, create a token that's a copy ... except it has changeling. Those tokens gain haste." Changeling
/// is part of each token's copiable values; haste is granted separately (CR 702.73a, 707.2, 707.9a, 611.2a).
fn infiltration(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let targets = targets::legal_objects(g, item);
    let copies: Vec<_> = targets
        .iter()
        .map(|&id| (g.copiable_values(id).with_keyword(Keyword::Changeling), item.controller))
        .collect();
    let tokens = g.create_token_copies_together(&copies)?;
    instruction(g, |g| {
        for &token in &tokens {
            cantrip_mechanics::effects::gain_haste(g, item.object, token);
        }
    });
    g.end_sentence();
    delayed::at_next_end_step_about(g, item, 0, &tokens);
    Ok(())
}

/// "Exile those tokens at the beginning of the next end step." Only the same token objects still on the battlefield
/// are exiled; tokens that changed zones are no longer those permanents (CR 603.7c, 400.7).
fn exile_created(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let created = g.event_objects(item).to_vec();
    let still_present: Vec<_> = created.into_iter().filter(|&id| targets::permanent(g, id)).collect();
    g.exile_all(&still_present);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE_AN_OPPONENT_CONTROLS.counted()], infiltration),
    delayed: &[delayed_trigger(exile_created, "Exile those tokens at the beginning of the next end step.")],
    ..oracle::DOMINION_INFILTRATION
};
