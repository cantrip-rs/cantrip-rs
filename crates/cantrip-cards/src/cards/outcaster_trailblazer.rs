use cantrip_api::*;
use cantrip_mechanics::plot::plot;
use cantrip_mechanics::{effects, triggers};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(triggers::THIS_ENTERS, effects::add_one_mana_of_any_color, "When this creature enters, add one mana of any color.").produces_mana(),
        triggered(
            when_asked(EventMask::ZONE_CHANGE, triggers::another_creature_you_control_with_power_between_enters::<4, { i32::MAX }>),
            |g, item| effects::draw(g, item, 1),
            "Whenever another creature you control with power 4 or greater enters, draw a card.",
        ),
        plot(
            mana("{2}{G}"),
            "Plot {2}{G} (You may pay {2}{G} and exile this card from your hand. Cast it as a sorcery on a later turn without paying its mana cost. Plot only as a sorcery.)",
        ),
    ],
    ..oracle::OUTCASTER_TRAILBLAZER
};
