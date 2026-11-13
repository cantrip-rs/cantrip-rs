use cantrip_api::*;
use cantrip_mechanics::{delayed, targets};
use cantrip_oracle as oracle;

/// "X can't be greater than the number of snow lands you control." (CR 601.2b).
fn snow_land_limit(q: &dyn Query, asker: &Asker, _: &[Target]) -> u32 {
    let snow_land = |id: ObjectId| q.controller(id) == asker.controller && q.chars(id).supertypes.snow && q.chars(id).is(CardType::Land);
    q.state().battlefield().iter().filter(|&&id| snow_land(id)).count() as u32
}

/// "Cast this spell only during combat before blockers are declared. X can't be greater than the number of snow lands you control. Choose X target attacking creatures. For each of those creatures, its controller may pay {1} or {2}. If that player doesn't, destroy that creature at end of combat. If that player pays only {1}, prevent all combat damage that would be dealt to and dealt by that creature this combat." (CR 608.2, 611.2a, 118.1).
fn cast_spell_only_during_combat(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    for creature in targets::legal_objects(g, item) {
        let payer = g.controller(creature);
        if g.may_pay(payer, item.object, creature, cost("{1}"), &PAY_ONE)? {
            let shield = EffectKind::Prevention { card: item.ability.card, index: 0, to: Target::None, shield: Shield::All };
            g.add_effect(item.object, creature, shield, Duration::EndOfCombat);
        } else if !g.may_pay(payer, item.object, creature, cost("{2}"), &PAY_TWO)? {
            delayed::at_end_of_combat(g, item, 0, Target::Object(creature));
        }
        g.end_sentence();
    }
    Ok(())
}

static PAY_ONE: Question = Question::new(
    "Pay",
    "pay {cost} to prevent combat damage to and by {subject}?",
    "If you decline, you may pay {2} instead",
    "Pay {cost}",
    "Decline",
);
static PAY_TWO: Question = Question::new(
    "Pay",
    "pay {cost} for {subject}?",
    "If you decline, it will be destroyed at end of combat",
    "Pay {cost}",
    "Decline",
);

pub static CARD: CardDef = CardDef {
    spell: spell(
        &[targets::ATTACKING_CREATURE.counted()],
        cast_spell_only_during_combat,
    ),
    delayed: &[delayed::destroy_that_at_end_of_combat("If that player doesn't, destroy that creature at end of combat.")],
    granted_preventions: &[
        cantrip_mechanics::prevention::COMBAT_DAMAGE_DEALT_TO_AND_BY
            .text("If that player pays only {1}, prevent all combat damage that would be dealt to and dealt by that creature this combat."),
    ],
    cast_only_if: Some(cantrip_mechanics::combat::cast_only_before_blockers),
    x_rule: Some(XRule::AtMost(snow_land_limit)),
    ..oracle::WINTERS_CHILL
};
