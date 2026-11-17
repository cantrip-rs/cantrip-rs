use cantrip_api::*;
use cantrip_mechanics::triggers::{activated_ability_on_stack, activation, THIS_ENTERS};
use cantrip_mechanics::{targets, tokens};
use cantrip_oracle as oracle;

/// "Whenever you activate an ability that targets a creature or player" (CR 602.2, 603.2, 115.1): an ability this ability's
/// controller activates that uses the stack (a mana ability has no targets, CR 605.1a), with a target that is a creature on the
/// battlefield or a player as it was activated; one that targets only a land, an artifact, a planeswalker or a card in a graveyard
/// does not. About that ability on the stack, to copy it.
fn you_activate_an_ability_that_targets_a_creature_or_player(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    let (_, player, _) = activation(q, event)?;
    if player != me.controller {
        return None;
    }
    let ability = activated_ability_on_stack(q, event)?;
    let item = q.state().stack.iter().find(|i| Target::Ability(i.serial) == ability)?;
    let creature_or_player = q.state().targets(item).iter().any(|&t| {
        matches!(t, Target::Player(_)) || t.object().is_some_and(|id| targets::permanent(q, id) && q.chars(id).is(CardType::Creature))
    });
    if creature_or_player { Fired::about(ability) } else { None }
}

/// "Copy that ability. You may choose new targets for the copy." (CR 707.10, 707.10c) The ability the trigger is about, copied even
/// if it has left the stack (as it last was there).
fn copy_it(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.copy_ability(item.subject, item.controller, true)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            THIS_ENTERS,
            tokens::create_a_mercenary,
            "When Ertha Jo enters, create a 1/1 red Mercenary creature token with \"{T}: Target creature you control gets +1/+0 until end of turn. Activate only as a sorcery.\"",
        ),
        triggered(
            when_asked(EventMask::ABILITY_ACTIVATED, you_activate_an_ability_that_targets_a_creature_or_player),
            copy_it,
            "Whenever you activate an ability that targets a creature or player, copy that ability. You may choose new targets for the copy.",
        ),
    ],
    ..oracle::ERTHA_JO_FRONTIER_MENTOR
};

pub static EXTRA: &[&CardDef] = &[&tokens::MERCENARY];
