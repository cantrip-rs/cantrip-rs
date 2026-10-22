use cantrip_api::*;
use cantrip_mechanics::{statics, tokens, triggers};
use cantrip_oracle as oracle;

/// A 1/1 black and green Insect creature token with flying (CR 111.4, 205.3m).
pub static INSECT: CardDef = CardDef {
    name: "Insect Token",
    token_name: Some(&[Subtype::Insect]),
    has_cost: false,
    // Black and green: bits 2 and 4 of W, U, B, R, G.
    colors: Some(ColorSet(0b10100)),
    types: TypeSet::of(&[CardType::Creature]),
    subtypes: SubtypeSet::of(&[Subtype::Insect]),
    power: Some(1),
    toughness: Some(1),
    keywords: Keywords::of(&[Keyword::Flying]),
    ..CardDef::BLANK
};

/// "Create two 1/1 black and green Insect creature tokens with flying." (CR 111.1.)
fn two_insects(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create(g, item.controller, &INSECT, 2)?;
    Ok(())
}

/// "Insects and Spiders you control" while delirium holds (CR 207.2c, 613.1f): creatures under the source's controller's control
/// with the Insect or Spider type as the earlier layers made them, the types read through the source's text (CR 612). The
/// source itself is a Scarecrow and is not among them.
fn insects_and_spiders_you_control_with_delirium(view: &dyn LayerQuery, source: ObjectId, affected: ObjectId, chars: &Chars) -> bool {
    let words = view.words(source);
    view.state().obj(affected).controller == view.state().obj(source).controller
        && chars.is(CardType::Creature)
        && (chars.has_subtype(words.subtype(Subtype::Insect)) || chars.has_subtype(words.subtype(Subtype::Spider)))
        && statics::delirium(view, source)
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(triggers::THIS_ENTERS, two_insects, "When The Swarmweaver enters, create two 1/1 black and green Insect creature tokens with flying.")],
    statics: &[
        static_ability(
            Layer::PtModify,
            insects_and_spiders_you_control_with_delirium,
            statics::plus_pt::<1, 1>,
            "Delirium \u{2014} As long as there are four or more card types among cards in your graveyard, Insects and Spiders you control get +1/+1.",
        ),
        static_ability(
            Layer::Ability,
            insects_and_spiders_you_control_with_delirium,
            statics::have_deathtouch,
            "Delirium \u{2014} As long as there are four or more card types among cards in your graveyard, Insects and Spiders you control have deathtouch.",
        ),
    ],
    ..oracle::THE_SWARMWEAVER
};

pub static EXTRA: &[&CardDef] = &[&INSECT];
