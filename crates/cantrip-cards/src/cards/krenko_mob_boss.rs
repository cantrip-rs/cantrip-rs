use cantrip_api::*;
use cantrip_mechanics::tokens;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        cost("{T}"),
        |g, item| tokens::create_x_where_x_is_the_number_you_control(g, item, &tokens::GOBLIN, Subtype::Goblin),
        "{T}: Create X 1/1 red Goblin creature tokens, where X is the number of Goblins you control.",
    )],
    ..oracle::KRENKO_MOB_BOSS
};

pub static EXTRA: &[&CardDef] = &[&tokens::GOBLIN];
