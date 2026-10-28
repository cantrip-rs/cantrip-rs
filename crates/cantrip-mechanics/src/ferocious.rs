//! Ferocious (CR 207.2c, an ability word with no rules meaning of its own): "if you control a creature with power 4 or
//! greater". Stubborn Denial, Fanatic of Rhonas and Bolt Bend print it, the last without the word, and Nighthowl
//! Pursuer and Ravening Warg as a trigger condition (`you_control_asked`); Temur Battlecrier and Dragonhawk, Fate's Tempest count such creatures.

use cantrip_api::*;

/// How many creatures `player` controls with power 4 or greater, as the game is now.
pub fn count(q: &dyn Query, player: PlayerId) -> usize {
    q.permanents_of(player)
        .into_iter()
        .filter(|&id| {
            let c = q.chars(id);
            c.is(CardType::Creature) && c.power.is_some_and(|p| p >= 4)
        })
        .count()
}

/// Whether `player` controls a creature with power 4 or greater, as the game is now.
pub fn you_control(q: &dyn Query, player: PlayerId) -> bool {
    count(q, player) > 0
}

/// `you_control` as a condition of the ability's controller, asked of an `Asker`: a triggered ability's "while you control a
/// creature with power 4 or greater", asked as the trigger event happens and not again on resolution (`AbilityDef::only_if`); its
/// intervening "if you control a creature with power 4 or greater" (`AbilityDef::condition`, CR 603.4), asked as it triggers and
/// again as it resolves; or an activation restriction (Fanatic of Rhonas). Nasty Little Rabbit, Wargling and Wilderland Scrounger
/// print them too.
pub fn you_control_asked(q: &dyn Query, asker: &Asker) -> bool {
    you_control(q, asker.controller)
}
