use cantrip_api::*;
use cantrip_mechanics::{damage_target, effects, targets, triggers};
use cantrip_oracle as oracle;

const ASK: Question = Question::new("DealDamage", "have this creature deal 3 damage to the target creature?", "If you do, it assigns no combat damage this turn", "Deal damage", "Don't").no_default();

/// The optional damage is a resolving cost; choosing it also prevents combat damage assignment (CR 118.12).
fn have_deal_3_damage_target(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if effects::ask_may(g, item.controller, item.object, Purpose::Ask(&ASK))? {
        damage_target(g, item, 0, 3);
        g.end_sentence();
        instruction(g, |g| {
            g.add_effect(item.object, item.object, EffectKind::NoCombatDamageAssignment, Duration::EndOfTurn);
        });
        g.end_sentence();
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
                when_asked(EventMask::UNBLOCKED, triggers::this_attacks_and_isnt_blocked),
                have_deal_3_damage_target,
                "Whenever this creature attacks and isn't blocked, you may have it deal 3 damage to target creature. If you do, this creature assigns no combat damage this turn.",
            )
            .targets(&[targets::CREATURE]),
    ],
    ..oracle::FARRELS_ZEALOT
};
