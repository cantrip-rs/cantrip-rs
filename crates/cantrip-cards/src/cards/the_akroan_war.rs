use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// II - "creatures your opponents control": the creatures the Saga's controller does not control, read as attackers are
/// declared (CR 611.2c), so a creature that changes hands or arrives later is judged as it then is.
fn creatures_your_opponents_control(q: &dyn Query, asker: &Asker, _: Target, id: ObjectId) -> bool {
    q.chars(id).is(CardType::Creature) && q.controller(id) != asker.controller
}

/// II - "Until your next turn, creatures your opponents control attack each combat if able." (CR 508.1d, 611.2c).
fn must_attack(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.add_lasting_rule(item, 0, Target::None, Duration::UntilNextTurnOf(item.controller));
    Ok(())
}

/// III - "Each tapped creature deals damage to itself equal to its power." (CR 120.3): every tapped creature on the
/// battlefield, whoever controls it, is its own source, so its own abilities count, and all are dealt damage together
/// (CR 101.4, 608.2c). A creature whose power is 0 or less deals none (CR 107.1b, 120.8).
fn tapped_creatures_hit_themselves(g: &mut dyn Game, _: &StackItem) -> Result<(), Suspend> {
    let mut hits: Vec<(ObjectId, Target, u32)> = Vec::new();
    let players: Vec<PlayerId> = g.state().apnap().collect();
    for player in players {
        for id in g.permanents_of(player) {
            if !g.obj(id).tapped || !g.chars(id).is(CardType::Creature) {
                continue;
            }
            let power = u32::try_from(g.chars(id).power.unwrap_or(0)).unwrap_or(0);
            hits.push((id, Target::Object(id), power));
        }
    }
    g.damage_together(&hits);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        chapter(1, effects::gain_control_of_target_while_source_on_battlefield, "I \u{2014} Gain control of target creature for as long as this Saga remains on the battlefield.")
            .targets(&[targets::CREATURE]),
        chapter(2, must_attack, "II \u{2014} Until your next turn, creatures your opponents control attack each combat if able."),
        chapter(3, tapped_creatures_hit_themselves, "III \u{2014} Each tapped creature deals damage to itself equal to its power."),
    ],
    lasting_rules: &[LastingRule {
        text: "Until your next turn, creatures your opponents control attack each combat if able.",
        rule: LastingRuleKind::MustAttack,
        applies: creatures_your_opponents_control,
    }],
    ..oracle::THE_AKROAN_WAR
};
