use cantrip_api::*;
use cantrip_mechanics::{effects, lasting_rules, triggers};
use cantrip_oracle as oracle;

/// "Untap each creature you control" (CR 701.26b), "after this phase, there is an additional combat phase" (CR 500.8): the
/// trigger comes as the declare blockers step is under way, so the extra combat phase follows the combat phase it is in.
fn untap_creatures_and_add_a_combat_phase(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::untap_all_creatures_you_control(g, item)?;
    g.add_combat_phase();
    Ok(())
}

/// "This creature must be blocked each combat this turn if able" (CR 509.1c, 611.2c): a requirement on the one object the
/// ability's words named, read at every declaration of blockers this turn; it is no longer on it once it is another object
/// (CR 400.7).
fn must_be_blocked_each_combat_this_turn(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.add_lasting_rule(item, 0, Target::Object(item.object), Duration::EndOfTurn);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            when_asked(EventMask::BECAME_BLOCKED, triggers::this_blocks_or_becomes_blocked),
            untap_creatures_and_add_a_combat_phase,
            "Whenever this creature becomes blocked, untap each creature you control. After this phase, there is an additional combat phase.",
        ),
        ability(cost("{3}{R}{R}{G}{G}"), must_be_blocked_each_combat_this_turn, "{3}{R}{R}{G}{G}: This creature must be blocked each combat this turn if able."),
    ],
    lasting_rules: &[LastingRule {
        text: "This creature must be blocked each combat this turn if able.",
        rule: LastingRuleKind::MustBeBlocked,
        applies: lasting_rules::the_object_named,
    }],
    ..oracle::ANZRAG_THE_QUAKE_MOLE
};
