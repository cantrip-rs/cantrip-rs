use cantrip_api::*;
use cantrip_mechanics::{earthbend, effects, triggers};
use cantrip_oracle as oracle;

/// "Only land creatures can attack during that combat phase." (CR 508.1c, 611.2c): a restriction on every creature that is not
/// also a land, asked at the declaration of attackers of the additional combat phase it was made for.
fn lasting_rule_0(q: &dyn Query, _: &Asker, _: Target, id: ObjectId) -> bool {
    q.chars(id).is(CardType::Creature) && !q.chars(id).is(CardType::Land)
}

/// "Untap all lands you control. After this phase, there is an additional combat phase. Only land creatures can attack during
/// that combat phase." (CR 701.26b, 500.8, 611.2a): the phase is added after the combat phase under way, and the restriction,
/// made now, is about that phase alone, not the combat phase this ability triggered in.
fn untap_lands_and_add_a_combat_phase(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::untap_all_lands_you_control(g, item)?;
    let phases = g.add_phases(PhaseAnchor::Current, &[PhaseSpec::Whole(Phase::Combat)]);
    if let Some(&phase) = phases.first() {
        g.add_lasting_rule(item, 0, Target::None, Duration::DuringCombatPhase(phase));
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        earthbend::enters::<4>("When Bumi enters, earthbend 4."),
        triggered(
            when(EventMask::DAMAGE, triggers::deals_combat_damage_to_player),
            untap_lands_and_add_a_combat_phase,
            "Whenever Bumi deals combat damage to a player, untap all lands you control. After this phase, there is an additional combat phase. Only land creatures can attack during that combat phase.",
        ),
    ],
    delayed: &[earthbend::RETURN],
    lasting_rules: &[LastingRule {
        text: "Only land creatures can attack during that combat phase.",
        rule: LastingRuleKind::CantAttack,
        applies: lasting_rule_0,
    }],
    ..oracle::BUMI_UNLEASHED
};
