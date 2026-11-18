use cantrip_api::*;
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

/// "Return another target creature card from your graveyard to your hand." (CR 400.7: it becomes a new object.)
fn return_it(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Some(card) = targets::object(g, item, 0) {
        g.return_to_hand(&[card]);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        cost("{3}{B}").and(EXILE_THIS),
        return_it,
        "{3}{B}, Exile this card from your graveyard: Return another target creature card from your graveyard to your hand.",
    )
    .targets(&[targets::ANOTHER_CREATURE_CARD_IN_YOUR_GRAVEYARD])
    .from(ZoneKind::Graveyard)],
    ..oracle::THEORETICAL_NECROMANCER
};
