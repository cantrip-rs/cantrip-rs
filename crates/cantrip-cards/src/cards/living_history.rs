use cantrip_api::*;
use cantrip_mechanics::{effects, history, targets, tokens, triggers};
use cantrip_oracle as oracle;

/// "Create a 2/2 red and white Spirit creature token."
fn spirit(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create(g, item.controller, &tokens::SPIRIT_2_2_RED_WHITE, 1)?;
    Ok(())
}

/// "Whenever you attack, if a card left your graveyard this turn, target attacking creature gets +2/+0 until end of turn." The
/// "if" is an intervening if (CR 603.4): it is checked as the attack is declared and again as the ability resolves, so a card
/// leaving the graveyard only after the ability is on the stack does not matter, nor does one that left before it. With no attacking
/// creature to target (an attack that is over), the ability is not put on the stack; one no longer legal does nothing (CR 608.2b).
pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(triggers::THIS_ENTERS, spirit, "When this enchantment enters, create a 2/2 red and white Spirit creature token."),
        triggered(
            triggers::YOU_ATTACK,
            effects::target_creature_gets::<2, 0>,
            "Whenever you attack, if a card left your graveyard this turn, target attacking creature gets +2/+0 until end of turn.",
        )
        .condition(history::a_card_left_your_graveyard_this_turn)
        .targets(&[targets::ATTACKING_CREATURE]),
    ],
    ..oracle::LIVING_HISTORY
};

pub static EXTRA: &[&CardDef] = &[&tokens::SPIRIT_2_2_RED_WHITE];
