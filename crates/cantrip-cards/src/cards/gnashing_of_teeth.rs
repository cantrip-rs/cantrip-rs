use cantrip_api::*;
use cantrip_mechanics::replacements::{EXILE_INSTEAD_OF_DYING, exile_if_it_would_die};
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// Mode 1: "Target creature gets -5/-5 until end of turn. If that creature would die this turn, exile it instead." The
/// replacement is the spell's own effect on that creature (CR 611.2c, 614.1a), made whether or not it is still legal.
fn shrink_and_exile(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    effects::target_creature_gets::<-5, -5>(g, item)?;
    g.end_sentence();
    exile_if_it_would_die(g, item.object, 0, creature);
    Ok(())
}

/// Mode 2: "Creatures target player controls get -1/-1 until end of turn." (CR 611.2c): the creatures that player controls as
/// it resolves, and no creature that enters later; nothing if the player is no longer a legal target (CR 608.2b).
fn shrink_their_creatures(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(player) = targets::player(g, item, 0) else { return Ok(()) };
    let creatures: Vec<ObjectId> = g.permanents_of(player).into_iter().filter(|&id| g.chars(id).is(CardType::Creature)).collect();
    instruction(g, |g| {
        for id in creatures {
            cantrip_mechanics::pump(g, item.object, id, -1, -1);
        }
    });
    Ok(())
}

static MODAL: ModalDef = modes(&[
    mode(shrink_and_exile, "Target creature gets -5/-5 until end of turn. If that creature would die this turn, exile it instead.")
        .targets(&[targets::CREATURE]),
    mode(shrink_their_creatures, "Creatures target player controls get -1/-1 until end of turn.").targets(&[targets::PLAYER]),
]);

pub static CARD: CardDef = CardDef {
    spell: Some(SpellDef::MODAL),
    modal: Some(&MODAL),
    granted_replacements: &[EXILE_INSTEAD_OF_DYING],
    ..oracle::GNASHING_OF_TEETH
};
