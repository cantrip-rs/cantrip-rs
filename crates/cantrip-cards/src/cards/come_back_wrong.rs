use cantrip_api::*;
use cantrip_mechanics::{delayed, targets};
use cantrip_oracle as oracle;

/// "Destroy target creature. If a creature card is put into a graveyard this way, return it to the battlefield under your
/// control. Sacrifice it at the beginning of your next end step." (CR 701.8a, 603.7): the creature is destroyed, and the card
/// it became returns only if it is in a graveyard and is a creature card there - a token is no card (CR 111.1, 111.7), a card a
/// replacement effect sent elsewhere is not in a graveyard, and a Vehicle or animated land that was a creature on the
/// battlefield is no creature card in the graveyard. It returns under the caster's control at once (the 2024-09-20 ruling: no
/// player acts between), and the delayed ability is about exactly that object (CR 603.7c, 400.7): this turn's end step, as
/// the spell resolves before it. Every creature card of a merged permanent that was put into a graveyard returns, together
/// (CR 730.3c), and the delayed ability sacrifices each.
fn destroy_and_return(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    let destroyed = g.destroy_all(&[creature]);
    let cards: Vec<(ObjectId, PlayerId, bool)> = destroyed
        .objects()
        .iter()
        .copied()
        .filter(|&id| !g.obj(id).token && g.obj(id).zone.is(ZoneKind::Graveyard) && g.chars(id).is(CardType::Creature))
        .map(|id| (id, item.controller, false))
        .collect();
    if cards.is_empty() {
        return Ok(());
    }
    let returned = g.put_all_onto_battlefield_as(&cards)?;
    delayed::at_your_next_end_step_about(g, item, 0, &returned);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE], destroy_and_return),
    delayed: &[delayed_trigger(delayed::sacrifice_created, "Sacrifice it at the beginning of your next end step.")],
    ..oracle::COME_BACK_WRONG
};
