use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "Whenever a player or permanent becomes the target of an ability you control" (CR 115.10, 603.2): a player, or a permanent
/// on the battlefield (not a spell or an ability on the stack, nor a card in another zone), targeted by an activated or
/// triggered ability, not a spell (`BecameTarget::ability`), that Loki's controller controls. Each thing a single ability
/// targets is its own event, so each would trigger it: "this ability triggers only once each turn" keeps the first.
fn player_or_permanent_becomes_the_target_of_your_ability(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    match event {
        Event::BecameTarget { target, controller, ability: true, .. }
            if *controller == me.controller
                && match target {
                    Target::Player(_) => true,
                    Target::Object(id) => targets::permanent(q, *id),
                    _ => false,
                } =>
        {
            Fired::PLAIN
        }
        _ => None,
    }
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        when_asked(EventMask::BECAME_TARGET, player_or_permanent_becomes_the_target_of_your_ability),
        |g, item| effects::draw(g, item, 1),
        "Whenever a player or permanent becomes the target of an ability you control, draw a card. This ability triggers only once each turn.",
    )
    .triggers_each_turn(1)],
    ..oracle::LOKI_GOD_OF_MISCHIEF
};
