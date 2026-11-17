use cantrip_api::*;
use cantrip_mechanics::{effects, targets::filters, tokens, triggers};
use cantrip_oracle as oracle;

/// A 1/1 black Vampire Rogue creature token with lifelink.
static VAMPIRE_ROGUE: CardDef = CardDef {
    colors: Some(ColorSet::of(Color::Black)),
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(1),
    toughness: Some(1),
    keywords: Keywords::of(&[Keyword::Lifelink]),
    ..token!(Vampire Rogue)
};

/// "Create a 1/1 black Vampire Rogue creature token with lifelink."
fn create_vampire_rogue(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create(g, item.controller, &VAMPIRE_ROGUE, 1)?;
    Ok(())
}

/// "Whenever one or more tokens you control enter" (CR 603.2c): one trigger per batch of tokens entering under this
/// creature's controller, and only the first such batch each turn ("This ability triggers only once each turn": no numbered
/// rule; CR 603.2h is "do this only once each turn"). "{1}{B}, Sacrifice another creature or
/// artifact" (CR 118.3, 701.21): the sacrifice is a cost, so the draw can't be stopped by removing what was sacrificed.
pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            when_asked(EventMask::ZONE_CHANGE, triggers::token_you_control_enters),
            create_vampire_rogue,
            "Whenever one or more tokens you control enter, create a 1/1 black Vampire Rogue creature token with lifelink. This ability triggers only once each turn.",
        )
        .triggers_each_turn(1)
        .once_per_batch(),
        ability(
            cost("{1}{B}").and(sacrifice(1, filters::another_creature_or_artifact)),
            |g, item| effects::draw(g, item, 1),
            "{1}{B}, Sacrifice another creature or artifact: Draw a card.",
        ),
    ],
    ..oracle::BARON_BERTRAM_GRAYWATER
};

pub static EXTRA: &[&CardDef] = &[&VAMPIRE_ROGUE];
