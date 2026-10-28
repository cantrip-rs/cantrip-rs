use cantrip_api::*;
use cantrip_mechanics::effects::plus_one_counter_on_this;
use cantrip_mechanics::tokens;
use cantrip_mechanics::triggers::{LANDFALL, THIS_ENTERS};
use cantrip_oracle as oracle;

/// Zabu, a legendary 2/2 green Cat creature token with "Landfall \u{2014} Whenever a land you control enters, put a +1/+1 counter on
/// Zabu." (CR 111.4, 205.4a, 207.2c): named by the effect that makes it, so it has the name Zabu and no mana cost.
pub static ZABU: CardDef = CardDef {
    name: "Zabu",
    has_cost: false,
    colors: Some(ColorSet::of(Color::Green)),
    types: TypeSet::of(&[CardType::Creature]),
    supertypes: Supertypes { basic: false, legendary: true, snow: false },
    subtypes: SubtypeSet::of(&[Subtype::Cat]),
    power: Some(2),
    toughness: Some(2),
    abilities: &[triggered(LANDFALL, plus_one_counter_on_this, "Landfall \u{2014} Whenever a land you control enters, put a +1/+1 counter on Zabu.")],
    text: "Landfall \u{2014} Whenever a land you control enters, put a +1/+1 counter on Zabu.",
    ..CardDef::BLANK
};

/// "When Ka-Zar enters, create Zabu, a legendary 2/2 green Cat creature token with ..." (CR 111.10a): one token, under its controller.
fn create_zabu(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create(g, item.controller, &ZABU, 1)?;
    Ok(())
}

/// "You may look at the top card of your library any time" (CR 401.2) and "You may play lands from the top of your library"
/// (CR 305.1): each land played that way still uses a land play (CR 305.2).
pub static CARD: CardDef = CardDef {
    play_rules: &[PlayRule::LookAtLibraryTop, PlayRule::LibraryTopLands],
    abilities: &[triggered(
        THIS_ENTERS,
        create_zabu,
        "When Ka-Zar enters, create Zabu, a legendary 2/2 green Cat creature token with \"Landfall \u{2014} Whenever a land you control enters, put a +1/+1 counter on Zabu.\"",
    )],
    ..oracle::KA_ZAR_OF_THE_SAVAGE_LAND
};

pub static EXTRA: &[&CardDef] = &[&ZABU];
