use cantrip_api::*;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_oracle as oracle;

/// The 1/1 blue Faerie Dragon creature token with flying.
pub static FAERIE_DRAGON: CardDef = CardDef {
    name: "Faerie Dragon Token",
    token_name: Some(&[Subtype::Faerie, Subtype::Dragon]),
    has_cost: false,
    colors: Some(ColorSet::of(Color::Blue)),
    types: TypeSet::of(&[CardType::Creature]),
    subtypes: SubtypeSet::of(&[Subtype::Faerie, Subtype::Dragon]),
    power: Some(1),
    toughness: Some(1),
    keywords: Keywords::of(&[Keyword::Flying]),
    text: "Flying",
    ..CardDef::BLANK
};

/// "If you have the initiative" (CR 603.4): asked as the ability would trigger and again as it resolves.
fn you_have_the_initiative(q: &dyn Query, asker: &Asker) -> bool {
    q.state().player(asker.controller).initiative
}

fn faerie_dragon(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    cantrip_mechanics::tokens::create(g, item.controller, &FAERIE_DRAGON, 1)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(THIS_ENTERS, cantrip_mechanics::effects::take_the_initiative, "When this creature enters, you take the initiative."),
        triggered(cantrip_mechanics::triggers::YOUR_END_STEP, faerie_dragon, "At the beginning of your end step, if you have the initiative, create a 1/1 blue Faerie Dragon creature token with flying.")
            .condition(you_have_the_initiative),
    ],
    ..oracle::FEYWILD_CARETAKER
};

pub static EXTRA: &[&CardDef] = &[&FAERIE_DRAGON];
