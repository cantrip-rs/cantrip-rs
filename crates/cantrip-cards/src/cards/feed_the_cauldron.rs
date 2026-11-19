use cantrip_api::*;
use cantrip_mechanics::{targets, tokens};
use cantrip_oracle as oracle;

/// "Destroy target creature with mana value 3 or less. If it's your turn, create a Food token." (CR 701.8, 608.2b, 111.10b): the
/// Food comes whether or not the creature was destroyed (indestructible), and whose turn it is is read as the spell resolves, so
/// one cast on an opponent's turn only destroys. With its only target illegal the spell does nothing at all, so no Food either.
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    g.destroy(creature);
    g.end_sentence();
    if g.state().turn.active == item.controller {
        tokens::create_food(g, item.controller)?;
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[target(targets::creature_with_mana_value_at_most::<3>)], effect),
    ..oracle::FEED_THE_CAULDRON
};

pub static EXTRA: &[&CardDef] = &[&tokens::FOOD];
