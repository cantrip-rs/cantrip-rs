use cantrip_api::*;
use cantrip_mechanics::max_speed::max_speed;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_mechanics::{library, tokens};
use cantrip_oracle as oracle;

pub static EXTRA: &[&CardDef] = &[&tokens::ZOMBIE];

pub static CARD: CardDef = CardDef {
    start_your_engines: true,
    abilities: &[triggered(THIS_ENTERS, tokens::create_a_zombie, "When this enchantment enters, create a 2/2 black Zombie creature token.")],
    delayed: &[ability(
        cost("{1}{B}").and(SACRIFICE_THIS),
        library::search_for_a_card_to_hand,
        "{1}{B}, Sacrifice this enchantment: Search your library for a card, put it into your hand, then shuffle. Activate only as a sorcery.",
    )
    .sorcery_speed()],
    statics: &[max_speed(
        "Max speed \u{2014} {1}{B}, Sacrifice this enchantment: Search your library for a card, put it into your hand, then shuffle. Activate only as a sorcery.",
    )],
    ..oracle::HOUR_OF_VICTORY
};
