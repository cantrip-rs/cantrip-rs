use cantrip_api::*;
use cantrip_mechanics::fight::targets_fight;
use cantrip_mechanics::{effects, select, targets};
use cantrip_oracle as oracle;

/// I: "Gain control of up to two target creatures with total mana value 6 or less for as long as this Saga remains on the
/// battlefield" (CR 611.2b, 108.4). The total was judged as the targets were chosen and is judged again on resolution (CR
/// 608.2b); a Saga that has already left the battlefield gives control for no time at all.
fn steal(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let creatures = targets::legal_objects(g, item);
    // One instruction for all of them (CR 608.2c).
    instruction(g, |g| {
        for creature in creatures {
            g.gain_control(item.object, creature, item.controller, Duration::WhileSourceOnBattlefield);
        }
    });
    Ok(())
}

/// II: "Creatures you control get +1/+1 and gain vigilance until end of turn."
fn rally(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::pump_creatures_you_control_and_gain(g, item, 1, 1, Keyword::Vigilance);
    Ok(())
}

/// "Up to one other target creature" (CR 115.1, 701.14c): a creature other than the one chosen for the first target, which
/// can't fight itself.
fn creature_other_than_the_first_target(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    (targets::CREATURE.filter)(q, asker, t) && !q.chosen_targets().contains(&t)
}

/// III: "Target creature you control fights up to one other target creature" (CR 701.14a). If either is an illegal target as
/// it resolves, neither fights (CR 701.14b, 608.2b).
fn fight(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    targets_fight(g, item, 0, 1);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        chapter(1, steal, "I \u{2014} Gain control of up to two target creatures with total mana value 6 or less for as long as this Saga remains on the battlefield.")
            .targets(&[TargetSpec { min: 0, max: 2, ..targets::CREATURE }
                .together(select::total_mana_value_at_most(6))
                .labelled("Creatures to gain control of")]),
        chapter(2, rally, "II \u{2014} Creatures you control get +1/+1 and gain vigilance until end of turn."),
        chapter(3, fight, "III \u{2014} Target creature you control fights up to one other target creature.")
            .targets(&[targets::CREATURE_YOU_CONTROL, target(creature_other_than_the_first_target).optional()]),
    ],
    ..oracle::THE_SUPER_HERO_CIVIL_WAR
};
