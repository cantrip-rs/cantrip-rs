use cantrip_api::*;
use cantrip_mechanics::{effects, targets, tokens, triggers};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(triggers::THIS_ENTERS, tokens::create_goblins::<1>, "When this creature enters, create a 1/1 red Goblin creature token."),
        ability(
            cost("{1}").and(TAP),
            effects::target_creature_cant_be_blocked_this_turn,
            "{1}, {T}: Target creature you control with power 2 or less can't be blocked this turn.",
        )
        .targets(&[targets::CREATURE_YOU_CONTROL_WITH_POWER_2_OR_LESS]),
    ],
    ..oracle::UNDERFOOT_UNDERDOGS
};

pub static EXTRA: &[&CardDef] = &[&tokens::GOBLIN];
