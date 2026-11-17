use cantrip_api::*;
use cantrip_mechanics::{effects, outlaw, targets, tokens};
use cantrip_oracle as oracle;

/// "Put a +1/+1 counter on this creature. If an outlaw was sacrificed this way, create a Treasure token": the creature this
/// activation's cost sacrificed (`Query::cost_objects`, CR 602.2b, 608.2h) is an outlaw by the creature types it last had on the
/// battlefield (CR 700.12, the rulings), which `Query::chars` reads of an object that has left.
fn counter_and_treasure_for_an_outlaw(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let outlaw_sacrificed = g.cost_objects(item).iter().any(|&creature| outlaw::is_outlaw(&g.chars(creature)));
    effects::plus_one_counter_on_this(g, item)?;
    g.end_sentence();
    if outlaw_sacrificed {
        tokens::create_a_treasure(g, item)?;
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        cost("{1}{B}").and(sacrifice(1, targets::filters::another_creature)),
        counter_and_treasure_for_an_outlaw,
        "{1}{B}, Sacrifice another creature: Put a +1/+1 counter on this creature. If an outlaw was sacrificed this way, create a Treasure token. (Assassins, Mercenaries, Pirates, Rogues, and Warlocks are outlaws.)",
    )],
    ..oracle::BONEYARD_DESECRATOR
};
