use cantrip_api::*;
use cantrip_mechanics::{choose, delayed, select, targets, triggers};
use cantrip_oracle as oracle;

/// Set up a trigger for this particular creature dying this turn (CR 603.7, 400.7).
fn watch_creature(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    delayed::this_turn_watching(g, item, creature);
    Ok(())
}

/// The spell's controller chooses from the dead creature's owner's graveyard; that owner controls the returned card.
fn return_creature(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = item.subject.object() else { return Ok(()) };
    let owner = g.obj(creature).owner;
    let graveyard = g.state().zone(Zone::graveyard(owner)).to_vec();
    let chosen = choose::select(g, item.controller, item.object, Purpose::PutOntoBattlefield,
        graveyard, select::creature_cards(select::as_many_as_can(1)))?;
    let Some(&card) = chosen.first() else { return Ok(()) };
    g.put_onto_battlefield(card, owner, false)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE], watch_creature),
    delayed: &[triggered(triggers::THIS_DIES, return_creature,
        "When that creature dies this turn, return a creature card from its owner's graveyard to the battlefield under the control of that creature's owner.")],
    ..oracle::REINCARNATION
};
