use cantrip_api::*;
use cantrip_mechanics::statics::{attacks_each_combat_if_able, itself};
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_mechanics::{targets, tokens};
use cantrip_oracle as oracle;

/// The Void, a legendary 5/5 black Horror Villain creature token with flying, indestructible, and "The Void attacks each combat if
/// able" (CR 111.4, 205.4a): named by the text that makes it, so no text change of it renames it (CR 612.2).
pub static THE_VOID: CardDef = CardDef {
    name: "The Void",
    has_cost: false,
    colors: Some(ColorSet::of(Color::Black)),
    types: TypeSet::of(&[CardType::Creature]),
    supertypes: Supertypes { basic: false, legendary: true, snow: false },
    subtypes: SubtypeSet::of(&[Subtype::Horror, Subtype::Villain]),
    power: Some(5),
    toughness: Some(5),
    keywords: Keywords::of(&[Keyword::Flying, Keyword::Indestructible]),
    statics: &[static_ability(Layer::Ability, itself, attacks_each_combat_if_able, "The Void attacks each combat if able.")],
    ..CardDef::BLANK
};

/// "When The Sentry enters, target opponent creates The Void" (CR 111.2, 701.7a): the token is created by that opponent, under their
/// control, as the ability resolves - nothing if they have left the game since (CR 608.2b).
fn opponent_creates_the_void(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Some(opponent) = targets::player(g, item, 0) {
        tokens::create(g, opponent, &THE_VOID, 1)?;
    }
    Ok(())
}

/// Flying, vigilance and indestructible are core keywords.
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        THIS_ENTERS,
        opponent_creates_the_void,
        "When The Sentry enters, target opponent creates The Void, a legendary 5/5 black Horror Villain creature token with flying, indestructible, and \"The Void attacks each combat if able.\"",
    )
    .targets(&[targets::OPPONENT])],
    ..oracle::THE_SENTRY_GOLDEN_GUARDIAN
};

pub static EXTRA: &[&CardDef] = &[&THE_VOID];
