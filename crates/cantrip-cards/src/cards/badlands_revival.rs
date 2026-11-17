use cantrip_api::*;
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

/// The card chosen for the "up to one target" of `spec`, if one was chosen and it is still legal (CR 115.1a, 601.2c, 608.2b): either
/// target may be left unchosen, so the slot of a target is not its spec.
fn chosen(g: &dyn Game, item: &StackItem, spec: u8) -> Option<ObjectId> {
    (0..item.target_count as usize).find(|&slot| g.state().spec(item, slot) == spec).and_then(|slot| targets::object(g, item, slot))
}

/// "Return up to one target creature card from your graveyard to the battlefield. Return up to one target permanent card from
/// your graveyard to your hand." (CR 115.3, 608.2c): two instructions in order. One card may be chosen for both targets; it
/// enters the battlefield first and is a new object (CR 400.7), no longer the second target, so it stays there.
fn return_them(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Some(creature) = chosen(g, item, 0) {
        g.put_onto_battlefield(creature, item.controller, false)?;
    }
    g.end_sentence();
    if let Some(permanent) = chosen(g, item, 1) {
        g.return_to_hand(&[permanent]);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(
        &[targets::CREATURE_CARD_IN_YOUR_GRAVEYARD.optional(), targets::PERMANENT_CARD_IN_YOUR_GRAVEYARD.optional()],
        return_them,
    ),
    ..oracle::BADLANDS_REVIVAL
};
