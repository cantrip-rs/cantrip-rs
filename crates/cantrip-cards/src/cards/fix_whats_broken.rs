use cantrip_api::*;
use cantrip_oracle as oracle;

/// "Return each artifact and creature card with mana value X from your graveyard to the battlefield", X being the life paid as an
/// additional cost: every such card as this resolves, put onto the battlefield together (CR 603.6a). A card's mana value in a
/// graveyard has X as 0 (CR 107.3), so X = 0 returns the cards whose mana value is 0.
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let x = item.x as u32;
    let cards: Vec<(ObjectId, PlayerId, bool)> = g
        .state()
        .zone(Zone::graveyard(item.controller))
        .iter()
        .copied()
        .filter(|&id| {
            let c = g.chars(id);
            (c.is(CardType::Artifact) || c.is(CardType::Creature)) && g.mana_value(id) == x
        })
        .map(|id| (id, item.controller, false))
        .collect();
    g.put_all_onto_battlefield_as(&cards)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[], effect),
    // As an additional cost to cast this spell, pay X life.
    additional_cost: Some(Cost { pay_x_life: true, ..Cost::NONE }),
    ..oracle::FIX_WHATS_BROKEN
};
