use cantrip_api::*;
use cantrip_mechanics::{effects, targets, tokens, top_of_library};
use cantrip_oracle as oracle;

const REVEAL_AND_PUT_INTO_HAND: Question = Question::new(
    "RevealTopCard",
    "reveal the top card and put it into your hand?",
    "You lose life equal to its mana value",
    "Put it into your hand",
    "Leave it on top",
);

/// "+1: Look at the top card of your library. You may reveal that card and put it into your hand. If you do, you lose life equal to
/// its mana value." (CR 701.20a, 608.2c, 119.3).
fn look_top_card_library_reveal(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let top = top_of_library(g, item.controller, 1);
    g.look_at(item.controller, &top);
    g.end_sentence();
    if top.is_empty() || !effects::ask_may(g, item.controller, item.object, Purpose::Ask(&REVEAL_AND_PUT_INTO_HAND))? {
        return Ok(());
    }
    g.reveal(item.controller, &top);
    g.end_sentence();
    let in_hand = g.return_to_hand_revealed(&top);
    g.end_sentence();
    if let Some(&card) = in_hand.first() {
        let life = g.mana_value(card) as i32;
        g.lose_life(item.controller,(life) as i32);
    }
    Ok(())
}

pub static VAMPIRE: CardDef = CardDef {
    types: TypeSet::of(&[CardType::Creature]),
    colors: Some(ColorSet::of(Color::Black)),
    power: Some(2),
    toughness: Some(3),
    keywords: Keywords::of(&[Keyword::Flying, Keyword::Lifelink]),
    ..token!(Vampire)
};

/// "−2: Create a 2/3 black Vampire creature token with flying and lifelink." (CR 111.1).
fn create_2_3_black_vampire(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create(g, item.controller, &VAMPIRE, 1)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(
                loyalty(1),
                look_top_card_library_reveal,
                "+1: Look at the top card of your library. You may reveal that card and put it into your hand. If you do, you lose life equal to its mana value.",
            ),
        ability(
                loyalty(-2),
                create_2_3_black_vampire,
                "−2: Create a 2/3 black Vampire creature token with flying and lifelink.",
            ),
        ability(
                loyalty(-7),
                |g, item| effects::damage_target_and_gain_life(g, item, 13, 13),
                "−7: Sorin deals 13 damage to any target. You gain 13 life.",
            )
            .targets(&[targets::ANY]),
    ],
    ..oracle::SORIN_THE_MIRTHLESS
};

pub static EXTRA: &[&CardDef] = &[&VAMPIRE];
