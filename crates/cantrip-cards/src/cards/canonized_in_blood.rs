use cantrip_api::*;
use cantrip_mechanics::{effects, history, targets, tokens, triggers};
use cantrip_oracle as oracle;

/// The 4/3 white and black Vampire Demon creature token with flying.
static VAMPIRE_DEMON: CardDef = CardDef {
    colors: Some(ColorSet(ColorSet::of(Color::White).0 | ColorSet::of(Color::Black).0)),
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(4),
    toughness: Some(3),
    keywords: Keywords::of(&[Keyword::Flying]),
    text: "Flying",
    ..token!(Vampire Demon)
};

/// "Create a 4/3 white and black Vampire Demon creature token with flying." (CR 111.1)
fn create_a_vampire_demon(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create(g, item.controller, &VAMPIRE_DEMON, 1)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            triggers::YOUR_END_STEP,
            effects::plus_one_counters_on_target::<1>,
            "At the beginning of your end step, if you descended this turn, put a +1/+1 counter on target creature you control.",
        )
        .condition(history::descended_this_turn)
        .targets(&[targets::CREATURE_YOU_CONTROL]),
        ability(
            cost("{5}{B}{B}").and(SACRIFICE_THIS),
            create_a_vampire_demon,
            "{5}{B}{B}, Sacrifice this enchantment: Create a 4/3 white and black Vampire Demon creature token with flying.",
        ),
    ],
    ..oracle::CANONIZED_IN_BLOOD
};

pub static EXTRA: &[&CardDef] = &[&VAMPIRE_DEMON];
