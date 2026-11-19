use cantrip_api::*;
use cantrip_mechanics::{targets, tokens, triggers};
use cantrip_oracle as oracle;

/// "If you control an enchanted creature" (CR 303.4b, 603.4): a creature with an Aura attached to it, whoever controls the Aura, judged
/// as the draw step begins and again as the ability resolves.
fn you_control_an_enchanted_creature(q: &dyn Query, asker: &Asker) -> bool {
    q.permanents_of(asker.controller).into_iter().any(|id| q.chars(id).is(CardType::Creature) && targets::is_enchanted(q, id))
}

/// "Create a Wicked Role token attached to target creature you control" (CR 111.10q, 303.7): attached to the creature if it is
/// still a legal target as the ability resolves (CR 608.2b).
fn wicked_role(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    tokens::create_role(g, &tokens::WICKED_ROLE, item.controller, creature)?;
    Ok(())
}

/// "You lose 1 life and you draw an additional card" (CR 119.3, 121.1): the life first, then the card; a player at 0 life loses only
/// as state-based actions are next checked (CR 704.5a), after both.
fn lose_a_life_and_draw(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.lose_life(item.controller, 1);
    g.draw(item.controller, 1);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            triggers::THIS_ENTERS,
            wicked_role,
            "When this enchantment enters, create a Wicked Role token attached to target creature you control.",
        )
        .targets(&[targets::CREATURE_YOU_CONTROL]),
        triggered(
            triggers::YOUR_DRAW_STEP,
            lose_a_life_and_draw,
            "At the beginning of your draw step, if you control an enchanted creature, you lose 1 life and you draw an additional card.",
        )
        .condition(you_control_an_enchanted_creature),
    ],
    ..oracle::LORD_SKITTERS_BLESSING
};

pub static EXTRA: &[&CardDef] = &[&tokens::WICKED_ROLE];
