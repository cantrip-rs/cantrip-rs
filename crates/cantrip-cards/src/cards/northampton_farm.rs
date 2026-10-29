use cantrip_api::*;
use cantrip_mechanics::{choose, select, targets};
use cantrip_oracle as oracle;

/// "Exile target creature you own" (CR 701.13, 607.2a): exiled with this land, for its third ability. A token ceases to exist
/// in exile (CR 111.8) and is never returned; its Auras fall off and its Equipment becomes unattached (CR 704.5m, 704.5n).
fn exile_it(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    g.exile_with(&[creature], g.linked(item), false);
    Ok(())
}

/// "Return a creature card exiled with this land to the battlefield under your control. Return each other card exiled with
/// this land to its owner's hand." (CR 607.2a, 400.3.) The land was sacrificed as a cost, and the exile link outlives it
/// (CR 113.7a). The creature card is chosen (the exiled cards are shown, only a creature card may be taken), and the rest go back to their owners' hands.
fn return_them(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let exiled = g.exiled_with(g.linked(item));
    let chosen = choose::select(g, item.controller, item.object, Purpose::PutOntoBattlefield, exiled.clone(), select::creature_cards(select::as_many_as_can(1)))?.first().copied();
    if let Some(card) = chosen {
        g.put_onto_battlefield(card, item.controller, false)?;
    }
    g.end_sentence();
    let others: Vec<ObjectId> = exiled.into_iter().filter(|&id| Some(id) != chosen).collect();
    g.return_to_hand(&others);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        mana_ability(TAP, add(Mana::C), "{T}: Add {C}."),
        ability(cost("{1}, {T}"), exile_it, "{1}, {T}: Exile target creature you own.").targets(&[targets::CREATURE_YOU_OWN]),
        ability(
            cost("{2}, {T}").and(SACRIFICE_THIS),
            return_them,
            "{2}, {T}, Sacrifice this land: Return a creature card exiled with this land to the battlefield under your control. Return each other card exiled with this land to its owner's hand.",
        ),
    ],
    ..oracle::NORTHAMPTON_FARM
};
