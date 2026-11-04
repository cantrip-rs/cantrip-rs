use cantrip_api::*;
use cantrip_mechanics::tokens;
use cantrip_oracle as oracle;

pub static CAMARID: CardDef = CardDef {
    types: TypeSet::of(&[CardType::Creature]),
    colors: Some(ColorSet::of(Color::Blue)),
    power: Some(1),
    toughness: Some(1),
    ..token!(Camarid)
};

/// "{1}{U}{U}, Sacrifice a blue creature: Create X 1/1 blue Camarid creature tokens, where X is the sacrificed creature's mana value." (CR 608.2).
fn create_x_1_1_blue(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let count = g.cost_objects(item).first().map_or(0, |&object| g.mana_value(object));
    tokens::create(g, item.controller, &CAMARID, count)?;
    Ok(())
}

/// "{1}{U}{U}, Sacrifice a blue creature: Create X 1/1 blue Camarid creature tokens, where X is the sacrificed creature's mana value." (CR 608.2).
fn create_x_1_1_blue_2(q: &dyn Query, asker: &Asker, id: ObjectId) -> bool {
    q.chars(id).colors.has(q.words(asker.source).color(Color::Blue))
        && q.chars(id).is(CardType::Creature)
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(
            cost("{1}{U}{U}").and(sacrifice(1, create_x_1_1_blue_2)),
            create_x_1_1_blue,
            "{1}{U}{U}, Sacrifice a blue creature: Create X 1/1 blue Camarid creature tokens, where X is the sacrificed creature's mana value.",
        ),
    ],
    ..oracle::HOMARID_SPAWNING_BED
};

pub static EXTRA: &[&CardDef] = &[&CAMARID];
