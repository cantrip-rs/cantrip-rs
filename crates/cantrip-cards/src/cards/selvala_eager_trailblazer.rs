use cantrip_api::*;
use cantrip_mechanics::mana::one_of_the_chosen_color_per_different_power;
use cantrip_mechanics::{tokens, triggers};
use cantrip_oracle as oracle;

// Vigilance is the core's, read from the card's JSON.
pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            when_asked(EventMask::SPELL_CAST, triggers::creature_spell_you_cast),
            tokens::create_a_mercenary,
            "Whenever you cast a creature spell, create a 1/1 red Mercenary creature token with \"{T}: Target creature you control gets +1/+0 until end of turn. Activate only as a sorcery.\"",
        ),
        mana_ability(
            TAP,
            add_by(one_of_the_chosen_color_per_different_power).choose_color(),
            "{T}: Choose a color. Add one mana of that color for each different power among creatures you control.",
        ),
    ],
    ..oracle::SELVALA_EAGER_TRAILBLAZER
};

pub static EXTRA: &[&CardDef] = &[&tokens::MERCENARY];
