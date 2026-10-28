use cantrip_api::*;
use cantrip_mechanics::{effects, ferocious, triggers};
use cantrip_oracle as oracle;

/// "Ferocious \u{2014} Whenever you attack while you control a creature with power 4 or greater, you draw a card and lose 1 life": asked
/// as the attack is declared, as part of the trigger event, and never again as the ability resolves (this is no intervening "if",
/// CR 603.4; the ruling). One trigger however many creatures attack; its controller draws, then loses the life.
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::YOU_ATTACK,
        effects::draw_a_card_and_lose_1_life,
        "Ferocious \u{2014} Whenever you attack while you control a creature with power 4 or greater, you draw a card and lose 1 life.",
    )
    .only_if(ferocious::you_control_asked)],
    ..oracle::THE_CHIEF_WARG
};
