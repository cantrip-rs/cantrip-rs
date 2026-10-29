use cantrip_api::*;
use cantrip_mechanics::prepare::become_prepared;
use cantrip_mechanics::{effects, targets, triggers::YOUR_SECOND_MAIN_PHASE};
use cantrip_oracle as oracle;

/// Scheming Silvertongue's prepare spell (CR 722): not a card here, only ever a copy.
pub static SIGN_IN_BLOOD: CardDef = CardDef {
    name: "Sign in Blood",
    cost: mana("{B}{B}"),
    types: TypeSet::of(&[CardType::Sorcery]),
    spell: spell(&[targets::PLAYER], effects::target_player_draws_and_loses_life::<2, 2>),
    text: "Target player draws two cards and loses 2 life.",
    ..CardDef::BLANK
};

/// "If you gained 2 or more life this turn" (CR 603.4): the total gained over the turn, however many events it took, asked as it
/// would trigger and again as it resolves.
fn you_gained_two_or_more_life_this_turn(q: &dyn Query, asker: &Asker) -> bool {
    q.history().life_gained(asker.controller) >= 2
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        YOUR_SECOND_MAIN_PHASE,
        become_prepared,
        "At the beginning of your second main phase, if you gained 2 or more life this turn, this creature becomes prepared.",
    )
    .condition(you_gained_two_or_more_life_this_turn)],
    prepare: Some(&SIGN_IN_BLOOD),
    ..oracle::SCHEMING_SILVERTONGUE
};

pub static EXTRA: &[&CardDef] = &[&SIGN_IN_BLOOD];
