use cantrip_api::*;
use cantrip_mechanics::{airbend_all, targets};
use cantrip_oracle as oracle;

/// "Choose up to one target creature, then airbend all other creatures." The target is chosen as the spell is cast; a
/// chosen target that is no longer legal stops the whole spell (CR 608.2b), so a spell that resolves had either none or a
/// legal one. Every other creature on the battlefield, anyone's, is airbended at once (CR 701.65a), its card castable by its
/// owner from exile for {2}.
///
/// "Until your next turn, your opponents can't cast spells from anywhere other than their hands." A rule about each player
/// who is an opponent as it resolves (CR 611.2c), lasting until the beginning of the caster's next turn (CR 611.2a): it
/// stops casting from exile, a graveyard, a library or the command zone, the cards just airbended included, and neither
/// activated abilities nor copies of spells, which are not cast (CR 101.2, 707.10).
///
/// "Exile Avatar's Wrath": its last instruction (CR 608.2c), so it goes to exile rather than to its owner's graveyard.
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let spared = targets::object(g, item, 0);
    let others: Vec<ObjectId> = g
        .state()
        .battlefield()
        .iter()
        .copied()
        .filter(|&id| Some(id) != spared && g.chars(id).is(CardType::Creature))
        .collect();
    airbend_all(g, &others);
    g.end_sentence();
    let opponents: Vec<PlayerId> = g.state().opponents(item.controller).collect();
    instruction(g, |g| {
        for opponent in opponents {
            g.add_player_effect(
                item.controller,
                item.object,
                EffectKind::CantCastSpellsFrom(opponent, ZoneKinds::all_but(&[ZoneKind::Hand])),
                Duration::UntilNextTurnOf(item.controller),
            );
        }
    });
    g.end_sentence();
    g.exile(item.object);
    Ok(())
}

pub static CARD: CardDef = CardDef { spell: spell(&[targets::CREATURE.optional()], effect), ..oracle::AVATARS_WRATH };
