use cantrip_api::*;
use cantrip_mechanics::{delayed, effects, lasting_rules, targets, triggers};
use cantrip_oracle as oracle;

/// "{G}{G}, {T}: Target creature can't be regenerated this turn. Damage that would be dealt to that creature this turn can't be
/// prevented or dealt instead to another permanent or player. When the creature dies this turn, exile the creature." Three
/// sentences (CR 608.2c), each an effect about the creature that outlives this permanent (CR 611.2a): it can't be regenerated
/// (CR 701.19c); no prevention effect (CR 615.12) nor redirection effect (CR 614.9) applies to damage that would be dealt to it
/// - judged at the recipient the damage would be dealt to now (CR 616.1f), so damage redirected onto it from elsewhere is held
/// there too; and a delayed triggered ability watches it die (CR 603.7c). A creature that leaves and returns is a new object
/// none of them is about (CR 400.7). A target no longer legal as this resolves does nothing (CR 608.2b).
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    instruction(g, |g| g.add_effect(item.object, creature, EffectKind::CantBeRegenerated, Duration::EndOfTurn));
    g.end_sentence();
    // Its two rules, `lasting_rules` 0 and 1: "can't be prevented" and "can't be dealt instead to another permanent or player".
    instruction(g, |g| {
        for rule in 0..2 {
            g.add_lasting_rule(item, rule, Target::Object(creature), Duration::EndOfTurn);
        }
    });
    g.end_sentence();
    delayed::this_turn_watching(g, item, creature);
    Ok(())
}

/// "When the creature dies this turn, exile the creature." (CR 603.7c, 400.7e): the card it became in its owner's graveyard,
/// if it is still there; a token has ceased to exist (CR 111.7).
fn exile_the_creature(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = item.subject.object() else { return Ok(()) };
    effects::exile_what_it_became(g, creature);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        cost("{G}{G}").and(TAP),
        effect,
        "{G}{G}, {T}: Target creature can't be regenerated this turn. Damage that would be dealt to that creature this turn can't be prevented or dealt instead to another permanent or player. When the creature dies this turn, exile the creature.",
    )
    .targets(&[targets::CREATURE])],
    // Each about the one creature the ability targeted, the object it was as the ability resolved (CR 400.7).
    lasting_rules: &[
        LastingRule {
            text: "Damage that would be dealt to that creature this turn can't be prevented.",
            rule: LastingRuleKind::DamageCantBePrevented,
            applies: lasting_rules::the_object_named,
        },
        LastingRule {
            text: "Damage that would be dealt to that creature this turn can't be dealt instead to another permanent or player.",
            rule: LastingRuleKind::DamageCantBeRedirected,
            applies: lasting_rules::the_object_named,
        },
    ],
    delayed: &[triggered(triggers::THIS_DIES, exile_the_creature, "When the creature dies this turn, exile the creature.")],
    ..oracle::WHIPPOORWILL
};
