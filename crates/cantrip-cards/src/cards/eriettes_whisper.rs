use cantrip_api::*;
use cantrip_mechanics::{effects, targets, tokens};
use cantrip_oracle as oracle;

/// "Target opponent discards two cards. Create a Wicked Role token attached to up to one target creature you control." (CR 608.2c,
/// 608.2b, 303.7): the discard is the opponent's choice and is done first. The Role goes on the second target, which is optional;
/// nothing is created when none was chosen or the creature is gone as the spell resolves, and the discard happens either way,
/// as it does when the opponent is the one who is no longer a legal target.
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::target_player_discards::<2>(g, item)?;
    let Some(creature) = targets::object(g, item, 1) else { return Ok(()) };
    tokens::create_role(g, &tokens::WICKED_ROLE, item.controller, creature)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::OPPONENT, targets::CREATURE_YOU_CONTROL.optional()], effect),
    ..oracle::ERIETTES_WHISPER
};

pub static EXTRA: &[&CardDef] = &[&tokens::WICKED_ROLE];
