use cantrip_api::*;
use cantrip_mechanics::{creatures_you_control, effects, haste_until_end_of_turn, tokens, triggers};
use cantrip_oracle as oracle;

/// "Whenever you activate a loyalty ability, create a 2/2 colorless Wizard Soldier creature token named Cadet." Any
/// permanent's loyalty ability, by this one's controller (CR 606.2, 603.2). The activation triggers it (CR 602.2), so the
/// trigger goes on the stack above the ability and the Cadet is on the battlefield before that ability resolves (the ruling).
fn make_cadet(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create(g, item.controller, &tokens::CADET, 1)?;
    Ok(())
}

/// +1: "Creatures you control get +1/+0 and gain haste until end of turn": the creatures there are as it resolves
/// (CR 611.2c), the Cadet this activation made among them.
fn plus_one(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    instruction(g, |g| {
        for id in effects::pump_creatures_you_control(g, item, 1, 0) {
            haste_until_end_of_turn(g, item.object, id);
        }
    });
    Ok(())
}

/// -2: "Discard your hand, then draw a card for each creature you control": the creatures are counted after the discard.
fn minus_two(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.discard_hand(item.controller);
    let creatures = creatures_you_control(g, item.controller);
    g.draw(item.controller, creatures as u32);
    Ok(())
}

/// -3: "Ajani deals 4 damage to each creature except for tokens you control": one damage event for every other creature,
/// dealt at once (CR 120.2, 608.2c). The tokens are judged as it resolves, the Cadet this activation made among them.
fn minus_three(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let hits: Vec<(ObjectId, Target, i64)> = g
        .state()
        .battlefield()
        .iter()
        .copied()
        .filter(|&id| g.chars(id).is(CardType::Creature) && !(g.obj(id).token && g.controller(id) == item.controller))
        .map(|id| (item.object, Target::Object(id), 4))
        .collect();
    g.damage_together(&hits);
    Ok(())
}

pub static EXTRA: &[&CardDef] = &[&tokens::CADET];

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            triggers::YOU_ACTIVATE_A_LOYALTY_ABILITY,
            make_cadet,
            "Whenever you activate a loyalty ability, create a 2/2 colorless Wizard Soldier creature token named Cadet.",
        ),
        ability(loyalty(1), plus_one, "+1: Creatures you control get +1/+0 and gain haste until end of turn."),
        ability(loyalty(-2), minus_two, "\u{2212}2: Discard your hand, then draw a card for each creature you control."),
        ability(loyalty(-3), minus_three, "\u{2212}3: Ajani deals 4 damage to each creature except for tokens you control."),
    ],
    ..oracle::AJANI_UNRELENTING
};
