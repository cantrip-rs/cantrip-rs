use cantrip_api::*;
use cantrip_mechanics::{targets, triggers};
use cantrip_oracle as oracle;

/// "Target creature defending player controls gets -2/-0 and loses flying until your next turn" (CR 611.2a, 613.4c, 613.1f): one
/// instruction of two continuous effects from this creature, both lasting until its controller's next turn begins (CR 611.2a), and
/// both doing nothing if the target is no longer a legal one as this resolves (CR 608.2b). The target filter reads the attacking creature from
/// the trigger's subject (CR 508.5), so the ability is `THIS_ATTACKS_SUBJECT`.
fn target_gets_minus_two_zero_and_loses_flying(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let creature = targets::object(g, item, 0);
    let until = Duration::UntilNextTurnOf(item.controller);
    instruction(g, |g| {
        if let Some(creature) = creature {
            g.add_effect(item.object, creature, EffectKind::Pump(-2, 0), until);
            g.add_effect(item.object, creature, EffectKind::LoseKeywords(Keywords::of(&[Keyword::Flying])), until);
        }
    });
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::THIS_ATTACKS_SUBJECT,
        target_gets_minus_two_zero_and_loses_flying,
        "Whenever this creature attacks, target creature defending player controls gets -2/-0 and loses flying until your next turn.",
    )
    .targets(&[targets::CREATURE_DEFENDING_PLAYER_CONTROLS])],
    ..oracle::FEAR_OF_FALLING
};
