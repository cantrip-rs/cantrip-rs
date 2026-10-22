use cantrip_api::*;
use cantrip_mechanics::{delayed, tokens, triggers};
use cantrip_oracle as oracle;

/// The 3/3 blue Angel creature token with flying.
pub static ANGEL: CardDef = CardDef {
    name: "Angel Token",
    token_name: Some(&[Subtype::Angel]),
    has_cost: false,
    colors: Some(ColorSet::of(Color::Blue)),
    types: TypeSet::of(&[CardType::Creature]),
    subtypes: SubtypeSet::of(&[Subtype::Angel]),
    power: Some(3),
    toughness: Some(3),
    keywords: Keywords::of(&[Keyword::Flying]),
    text: "Flying",
    ..CardDef::BLANK
};

fn make_angel(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create(g, item.controller, &ANGEL, 1)?;
    Ok(())
}

/// "If you've drawn three or more cards this turn" (CR 603.4), read from this turn's history.
fn drew_three_or_more(q: &dyn Query, asker: &Asker) -> bool {
    q.history().drawn(asker.controller) >= 3
}

fn draw_two(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.draw(item.controller, 2);
    Ok(())
}

/// The resolving activation creates a delayed trigger lasting this turn (CR 603.7b),
/// independent of Lyra retaining her abilities or controller.
fn grant_draw_two(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    delayed::this_turn(g, item, item.object);
    Ok(())
}
pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            triggers::EACH_END_STEP,
            make_angel,
            "At the beginning of each end step, if you've drawn three or more cards this turn, create a 3/3 blue Angel creature token with flying.",
        )
        .condition(drew_three_or_more),
        ability(
            cost("{3}{U}{U}"),
            grant_draw_two,
            "{3}{U}{U}: Until end of turn, whenever Lyra deals combat damage to a player, draw two cards.",
        ),
    ],
    delayed: &[triggered(
        triggers::DEALS_COMBAT_DAMAGE_TO_PLAYER,
        draw_two,
        "Whenever Lyra deals combat damage to a player, draw two cards.",
    )],
    ..oracle::LYRA_TOLARIAN_ARCHANGEL
};

pub static EXTRA: &[&CardDef] = &[&ANGEL];
