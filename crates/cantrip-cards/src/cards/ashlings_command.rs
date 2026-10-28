use cantrip_api::*;
use cantrip_mechanics::{copy, effects, lands, targets, tokens};
use cantrip_oracle as oracle;

/// "Draw two cards" for the target player (see `effects::target_player_draws`).
fn draw_two(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::target_player_draws(g, item, 0, 2);
    Ok(())
}

/// "Ashling's Command deals 2 damage to each creature target player controls": the creatures that player controls as it
/// resolves, dealt the damage together (CR 120.2, 608.2c); nothing at all if the player is no longer a legal target
/// (CR 608.2b).
fn two_damage_to_each_creature(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(player) = targets::player(g, item, 0) else { return Ok(()) };
    let hits: Vec<(ObjectId, Target, u16)> = g
        .permanents_of(player)
        .into_iter()
        .filter(|&id| g.chars(id).is(CardType::Creature))
        .map(|id| (item.object, Target::Object(id), 2))
        .collect();
    g.damage_together(&hits);
    Ok(())
}

/// "Target player creates two Treasure tokens" (see `tokens::target_player_creates_treasures`).
fn two_treasures(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::target_player_creates_treasures(g, item, 0, 2)
}

static MODAL: ModalDef = modes(&[
    mode(copy::create_token_copy_of_target, "Create a token that's a copy of target Elemental you control.")
        .targets(&[target(targets::permanent_of_subtype_you_control::<{ lands::sub(Subtype::Elemental) }>)]),
    mode(draw_two, "Target player draws two cards.").targets(&[targets::PLAYER]),
    mode(two_damage_to_each_creature, "Ashling's Command deals 2 damage to each creature target player controls.")
        .targets(&[targets::PLAYER]),
    mode(two_treasures, "Target player creates two Treasure tokens.").targets(&[targets::PLAYER]),
])
.choose(2, 2);

pub static CARD: CardDef = CardDef { spell: Some(SpellDef::MODAL), modal: Some(&MODAL), ..oracle::ASHLINGS_COMMAND };

pub static EXTRA: &[&CardDef] = &[&tokens::TREASURE];
