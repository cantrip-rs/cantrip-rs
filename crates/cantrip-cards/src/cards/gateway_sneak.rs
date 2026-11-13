use cantrip_api::*;
use cantrip_mechanics::{effects, lands, triggers};
use cantrip_oracle as oracle;

/// "Whenever a Gate you control enters, this creature can't be blocked this turn." (CR 603.2, 509.1b): any Gate under its
/// controller's control as it enters, a land or a token or not. "Whenever this creature deals combat damage to a player, draw a
/// card." (CR 510.2, 121.1).
pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            when_asked(EventMask::ZONE_CHANGE, triggers::of_subtype_you_control_enters::<{ lands::sub(Subtype::Gate) }>),
            effects::this_cant_be_blocked_this_turn,
            "Whenever a Gate you control enters, this creature can't be blocked this turn.",
        ),
        triggered(triggers::DEALS_COMBAT_DAMAGE_TO_PLAYER, |g, item| effects::draw(g, item, 1), "Whenever this creature deals combat damage to a player, draw a card."),
    ],
    ..oracle::GATEWAY_SNEAK
};
