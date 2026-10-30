use cantrip_api::*;
use cantrip_mechanics::{copy, effects, lands, pump, targets};
use cantrip_oracle as oracle;

/// "Creatures target player controls get +1/+1 and gain haste until end of turn" (CR 611.2c): the creatures that player
/// controls as it resolves, and no creature that enters later; nothing at all if the player is no longer a legal target
/// (CR 608.2b).
fn team_pump(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(player) = targets::player(g, item, 0) else { return Ok(()) };
    let creatures: Vec<ObjectId> = g.permanents_of(player).into_iter().filter(|&id| g.chars(id).is(CardType::Creature)).collect();
    instruction(g, |g| {
        for creature in creatures {
            pump(g, item.object, creature, 1, 1);
            effects::gain_until_end_of_turn(g, item.object, creature, Keyword::Haste);
        }
    });
    Ok(())
}

/// "Target player mills five cards, then puts each Goblin card milled this way into their hand" (CR 701.17): the Goblin cards
/// are those the mill put into the graveyard (or wherever a replacement effect sent them), judged by the types they have
/// there (CR 205.3), and the player is the one who mills and who takes them.
fn mill_five_take_goblins(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(player) = targets::player(g, item, 0) else { return Ok(()) };
    let goblin = g.words(item.object).subtype(Subtype::Goblin);
    let milled = g.mill(player, 5);
    let goblins: Vec<ObjectId> = milled.into_iter().filter(|&card| g.chars(card).has_subtype(goblin)).collect();
    g.return_to_hand(&goblins);
    Ok(())
}

/// "Choose two" (CR 700.2): the modes are carried out in the order printed (CR 700.2d).
static MODAL: ModalDef = modes(&[
    mode(copy::create_token_copy_of_target, "Create a token that's a copy of target Goblin you control.")
        .targets(&[target(targets::permanent_of_subtype_you_control::<{ lands::sub(Subtype::Goblin) }>)]),
    mode(team_pump, "Creatures target player controls get +1/+1 and gain haste until end of turn.").targets(&[targets::PLAYER]),
    mode(effects::destroy_first_target, "Destroy target artifact or creature.").targets(&[targets::ARTIFACT_OR_CREATURE]),
    mode(mill_five_take_goblins, "Target player mills five cards, then puts each Goblin card milled this way into their hand.")
        .targets(&[targets::PLAYER]),
])
.choose(2, 2);

pub static CARD: CardDef = CardDef { spell: Some(SpellDef::MODAL), modal: Some(&MODAL), ..oracle::GRUBS_COMMAND };
