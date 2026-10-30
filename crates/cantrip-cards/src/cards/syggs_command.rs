use cantrip_api::*;
use cantrip_mechanics::{copy, effects, lands, targets};
use cantrip_oracle as oracle;

/// "Creatures target player controls gain lifelink until end of turn" (CR 611.2c): the creatures that player controls as it
/// resolves, and no creature that enters later; nothing at all if the player is no longer a legal target (CR 608.2b).
fn team_lifelink(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(player) = targets::player(g, item, 0) else { return Ok(()) };
    let creatures: Vec<ObjectId> = g.permanents_of(player).into_iter().filter(|&id| g.chars(id).is(CardType::Creature)).collect();
    instruction(g, |g| {
        for creature in creatures {
            effects::gain_until_end_of_turn(g, item.object, creature, Keyword::Lifelink);
        }
    });
    Ok(())
}

/// "Target player draws a card."
fn draw_one(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::target_player_draws(g, item, 0, 1);
    Ok(())
}

/// "Choose two" (CR 700.2): the modes are carried out in the order printed (CR 700.2d).
static MODAL: ModalDef = modes(&[
    mode(copy::create_token_copy_of_target, "Create a token that's a copy of target Merfolk you control.")
        .targets(&[target(targets::permanent_of_subtype_you_control::<{ lands::sub(Subtype::Merfolk) }>)]),
    mode(team_lifelink, "Creatures target player controls gain lifelink until end of turn.").targets(&[targets::PLAYER]),
    mode(draw_one, "Target player draws a card.").targets(&[targets::PLAYER]),
    mode(effects::tap_and_stun, "Tap target creature. Put a stun counter on it.").targets(&[targets::CREATURE]),
])
.choose(2, 2);

pub static CARD: CardDef = CardDef { spell: Some(SpellDef::MODAL), modal: Some(&MODAL), ..oracle::SYGGS_COMMAND };
