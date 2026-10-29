use cantrip_api::*;
use cantrip_mechanics::{choose, select, targets};
use cantrip_oracle as oracle;

/// "Heated Argument deals 6 damage to target creature. You may exile a card from your graveyard. If you do, Heated Argument also
/// deals 2 damage to that creature's controller." With its target illegal the spell does not resolve at all (CR 608.2b), so no card
/// is exiled and nobody is dealt damage. Otherwise the controller is read as the 2 damage is dealt, which is the last-known
/// controller if the creature is gone by then (CR 608.2h); a card exiled counts even if the damage is prevented.
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    g.damage(item.object, Target::Object(creature), 6);
    g.end_sentence();
    let graveyard = g.state().zone(Zone::graveyard(item.controller)).to_vec();
    let exiled = choose::select(g, item.controller, item.object, Purpose::Exile, graveyard, select::cards(select::up_to(1)))?;
    let Some(&card) = exiled.first() else { return Ok(()) };
    g.exile(card);
    g.end_sentence();
    let controller = g.controller(creature);
    g.damage(item.object, Target::Player(controller), 2);
    Ok(())
}

pub static CARD: CardDef = CardDef { spell: spell(&[targets::CREATURE], effect), ..oracle::HEATED_ARGUMENT };
