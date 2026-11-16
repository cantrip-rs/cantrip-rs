use cantrip_api::*;
use cantrip_mechanics::{tokens, triggers};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(triggers::THIS_ENTERS_OR_ATTACKS, tokens::create_a_squirrel, "Whenever The Unbeatable Squirrel Girl enters or attacks, create a 1/1 green Squirrel creature token."),
        ability(
            cost("{1}{G}{G}{G}"),
            |g, item| tokens::create_x_where_x_is_the_number_you_control(g, item, &tokens::SQUIRREL, Subtype::Squirrel),
            "{1}{G}{G}{G}: Create X 1/1 green Squirrel creature tokens, where X is the number of Squirrels you control.",
        ),
    ],
    ..oracle::THE_UNBEATABLE_SQUIRREL_GIRL
};

pub static EXTRA: &[&CardDef] = &[&tokens::SQUIRREL];
