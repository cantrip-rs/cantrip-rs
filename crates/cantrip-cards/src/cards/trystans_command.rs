use cantrip_api::*;
use cantrip_mechanics::{copy, destroy_target, lands, pump, targets};
use cantrip_oracle as oracle;

/// "Return one or two target permanent cards from your graveyard to your hand": the targets still legal (CR 608.2b).
fn return_permanent_cards(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.return_to_hand(&targets::legal_objects(g, item));
    Ok(())
}

/// "Destroy target creature or enchantment."
fn destroy_creature_or_enchantment(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    destroy_target(g, item, 0);
    Ok(())
}

/// "Creatures target player controls get +3/+3 until end of turn. Untap them." (CR 611.2c): the creatures that player controls as
/// it resolves, and no creature that enters later; "them" is exactly those. Nothing at all if the player is no longer a legal
/// target (CR 608.2b).
fn team_pump_and_untap(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(player) = targets::player(g, item, 0) else { return Ok(()) };
    let creatures: Vec<ObjectId> = g.permanents_of(player).into_iter().filter(|&id| g.chars(id).is(CardType::Creature)).collect();
    instruction(g, |g| {
        for &creature in &creatures {
            pump(g, item.object, creature, 3, 3);
        }
    });
    g.end_sentence();
    for creature in creatures {
        g.untap(creature);
    }
    Ok(())
}

/// "Choose two" (CR 700.2): the modes are carried out in the order printed (CR 700.2d).
static MODAL: ModalDef = modes(&[
    mode(copy::create_token_copy_of_target, "Create a token that's a copy of target Elf you control.")
        .targets(&[target(targets::permanent_of_subtype_you_control::<{ lands::sub(Subtype::Elf) }>)]),
    mode(return_permanent_cards, "Return one or two target permanent cards from your graveyard to your hand.")
        .targets(&[targets::PERMANENT_CARD_IN_YOUR_GRAVEYARD.between(1, 2)]),
    mode(destroy_creature_or_enchantment, "Destroy target creature or enchantment.").targets(&[target(targets::creature_or_enchantment)]),
    mode(team_pump_and_untap, "Creatures target player controls get +3/+3 until end of turn. Untap them.").targets(&[targets::PLAYER]),
])
.choose(2, 2);

pub static CARD: CardDef = CardDef { spell: Some(SpellDef::MODAL), modal: Some(&MODAL), ..oracle::TRYSTANS_COMMAND };
