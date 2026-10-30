use cantrip_api::*;
use cantrip_mechanics::{copy, fight, lands, pump, targets, tokens};
use cantrip_oracle as oracle;

/// "Target player creates a 1/1 green and white Kithkin creature token": under that player's control, nothing at all if the
/// player is no longer a legal target (CR 608.2b).
fn player_makes_kithkin(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(player) = targets::player(g, item, 0) else { return Ok(()) };
    tokens::create(g, player, &tokens::KITHKIN, 1)?;
    Ok(())
}

/// "Target creature you control gets +3/+3 until end of turn."
fn plus_three(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    instruction(g, |g| pump(g, item.object, creature, 3, 3));
    Ok(())
}

/// "Target creature you control fights target creature an opponent controls." (CR 701.14a-b): neither deals damage when either
/// target is no longer legal.
fn fight_targets(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    fight::targets_fight(g, item, 0, 1);
    Ok(())
}

/// "Choose two" (CR 700.2): the modes are carried out in the order printed (CR 700.2d), so a token made by the second mode
/// can be pumped by the third only if it was chosen as a target, which it cannot be.
static MODAL: ModalDef = modes(&[
    mode(copy::create_token_copy_of_target, "Create a token that's a copy of target Kithkin you control.")
        .targets(&[target(targets::permanent_of_subtype_you_control::<{ lands::sub(Subtype::Kithkin) }>)]),
    mode(player_makes_kithkin, "Target player creates a 1/1 green and white Kithkin creature token.").targets(&[targets::PLAYER]),
    mode(plus_three, "Target creature you control gets +3/+3 until end of turn.").targets(&[targets::CREATURE_YOU_CONTROL]),
    mode(fight_targets, "Target creature you control fights target creature an opponent controls.")
        .targets(&[targets::CREATURE_YOU_CONTROL, targets::CREATURE_AN_OPPONENT_CONTROLS]),
])
.choose(2, 2);

pub static CARD: CardDef = CardDef { spell: Some(SpellDef::MODAL), modal: Some(&MODAL), ..oracle::BRIGIDS_COMMAND };

pub static EXTRA: &[&CardDef] = &[&tokens::KITHKIN];
