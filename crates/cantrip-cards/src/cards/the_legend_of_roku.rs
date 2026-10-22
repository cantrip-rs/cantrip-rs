use cantrip_api::*;
use cantrip_mechanics::{firebending, tokens};
use cantrip_mechanics::{exile_and_return_transformed, library::exile_top_play_until_end_of_next_turn};
use cantrip_oracle as oracle;

/// I: "Exile the top three cards of your library. Until the end of your next turn, you may play those cards." The
/// three leave together, face up; each that reaches exile may be played by the ability's controller, even once they no
/// longer control the Saga, until their next turn's cleanup step. A library holding fewer than three exiles what it
/// has.
fn chapter_one(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    exile_top_play_until_end_of_next_turn(g, item.controller, 3);
    Ok(())
}

/// III: "Exile this Saga, then return it to the battlefield transformed under your control": the ability's
/// controller's, who need not own it.
fn chapter_three(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    exile_and_return_transformed(g, item.object, item.controller)
}

// --- Avatar Roku ------------------------------------------------------------------------------------------------

const FIREBENDING_4: AbilityDef = firebending::trigger::<4>("Firebending 4 (Whenever this creature attacks, add {R}{R}{R}{R}. This mana lasts until end of combat.)");

/// The 4/4 red Dragon with flying and firebending 4.
pub static DRAGON: CardDef = CardDef {
    name: "Dragon Token",
    token_name: Some(&[Subtype::Dragon]),
    has_cost: false,
    colors: Some(ColorSet::of(Color::Red)),
    types: TypeSet::of(&[CardType::Creature]),
    subtypes: SubtypeSet::of(&[Subtype::Dragon]),
    power: Some(4),
    toughness: Some(4),
    keywords: Keywords::of(&[Keyword::Flying]),
    abilities: &[FIREBENDING_4],
    text: "Flying\nFirebending 4",
    ..CardDef::BLANK
};

fn make_dragon(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create(g, item.controller, &DRAGON, 1)?;
    Ok(())
}

pub static BACK: CardDef = CardDef {
    abilities: &[
        FIREBENDING_4,
        ability(cost("{8}"), make_dragon, "{8}: Create a 4/4 red Dragon creature token with flying and firebending 4."),
    ],
    ..oracle::AVATAR_ROKU
};

pub static CARD: CardDef = CardDef {
    abilities: &[
        chapter(1, chapter_one, "I \u{2014} Exile the top three cards of your library. Until the end of your next turn, you may play those cards."),
        chapter(2, cantrip_mechanics::effects::add_one_mana_of_any_color, "II \u{2014} Add one mana of any color."),
        chapter(3, chapter_three, "III \u{2014} Exile this Saga, then return it to the battlefield transformed under your control."),
    ],
    back: Some(&BACK),
    ..oracle::THE_LEGEND_OF_ROKU
};

pub static EXTRA: &[&CardDef] = &[&DRAGON];
