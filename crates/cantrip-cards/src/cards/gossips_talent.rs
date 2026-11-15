use cantrip_api::*;
use cantrip_mechanics::{class, effects, targets, triggers};
use cantrip_oracle as oracle;

/// "Whenever you attack" (CR 508.1m), once the Class is level 2 (CR 716.2a).
fn you_attack_at_level_two(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    if class::level(q, me.source) >= 2 { triggers::you_attack(q, me, event) } else { None }
}

/// "Target attacking creature with power 3 or less" (CR 115.1, 506.4, 208.1): attacking, and its power as the layers make it
/// at most 3, judged as it is chosen and again as the ability resolves (CR 608.2b).
fn attacking_creature_with_power_3_or_less(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    targets::attacking_creature_filter(q, asker, t) && targets::creature_with_power_at_most::<3>(q, asker, t)
}

const ATTACKING_CREATURE_WITH_POWER_3_OR_LESS: TargetSpec = target(attacking_creature_with_power_3_or_less);

/// "Whenever a creature you control deals combat damage to a player" (CR 510.2), once the Class is level 3, the creature the
/// trigger's subject ("it"): one trigger for each creature and player.
fn creature_deals_combat_damage_at_level_three(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    if class::level(q, me.source) < 3 {
        return None;
    }
    triggers::creature_you_control_deals_combat_damage_to_player(q, me, event)?;
    let Event::Damage { source: from, .. } = event else { return None };
    Fired::about(Target::Object(*from))
}

/// "You may exile it. If you do, return it to the battlefield under its owner's control": the trigger's subject creature.
fn may_exile_the_subject_then_return_it(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = item.subject.object() else { return Ok(()) };
    effects::may_exile_then_return(g, item, creature)
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(triggers::CREATURE_YOU_CONTROL_ENTERS, |g, item| effects::surveil(g, item, 1), "Whenever a creature you control enters, surveil 1."),
        class::level_two("{1}{U}", "{1}{U}: Level 2"),
        triggered(
            when_asked(EventMask::ATTACKERS_DECLARED, you_attack_at_level_two),
            effects::target_creature_cant_be_blocked_this_turn,
            "Whenever you attack, target attacking creature with power 3 or less can't be blocked this turn.",
        )
        .targets(&[ATTACKING_CREATURE_WITH_POWER_3_OR_LESS]),
        class::level_three("{3}{U}", "{3}{U}: Level 3"),
        triggered(
            when_asked(EventMask::DAMAGE, creature_deals_combat_damage_at_level_three),
            may_exile_the_subject_then_return_it,
            "Whenever a creature you control deals combat damage to a player, you may exile it, then return it to the battlefield under its owner's control.",
        ),
    ],
    ..oracle::GOSSIPS_TALENT
};
