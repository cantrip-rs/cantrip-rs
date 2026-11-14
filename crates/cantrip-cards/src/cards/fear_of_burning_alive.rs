use cantrip_api::*;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_mechanics::{delirium, effects, targets};
use cantrip_oracle as oracle;

/// The intervening "if" (CR 603.4): delirium, checked as the ability triggers and again as it resolves.
fn has_delirium(q: &dyn Query, asker: &Asker) -> bool {
    delirium(q, asker.controller)
}

/// "Whenever a source you control deals noncombat damage to an opponent" (CR 120.2, 603.2): damage of more than zero that is not
/// combat damage, dealt by a source whose controller is this ability's controller (CR 120.2b: an object deals it) - a spell or a
/// permanent - to one of that player's opponents. One trigger for each source and each opponent it hurt, with the opponent the
/// trigger's subject ("that player"); the ability is `.damage_amount()`, which carries "that amount" for the resolving ability.
fn source_you_control_deals_noncombat_damage_to_an_opponent(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    match event {
        Event::Damage { target: Target::Player(p), amount, kind, controller, .. }
            if !kind.is_combat() && *amount > 0 && *controller == me.controller && q.state().opponents(me.controller).any(|o| o == *p) =>
        {
            Fired::about(Target::Player(*p))
        }
        _ => None,
    }
}

/// "Target creature that player controls", the player the damage was dealt to being the event's subject.
fn creature_that_player_controls(q: &dyn Query, _: &Asker, t: Target) -> bool {
    matches!(t, Target::Object(id) if targets::controlled_by_that_player(q, id) && q.chars(id).is(CardType::Creature))
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(THIS_ENTERS, effects::damage_each_opponent_n::<4>, "When this creature enters, it deals 4 damage to each opponent."),
        triggered(
            when_asked(EventMask::DAMAGE, source_you_control_deals_noncombat_damage_to_an_opponent),
            effects::this_deals_that_much_damage_to_target,
            "Delirium \u{2014} Whenever a source you control deals noncombat damage to an opponent, if there are four or more card types among cards in your graveyard, this creature deals that amount of damage to target creature that player controls.",
        )
        .targets(&[target(creature_that_player_controls)])
        .condition(has_delirium)
        .damage_amount(),
    ],
    ..oracle::FEAR_OF_BURNING_ALIVE
};
