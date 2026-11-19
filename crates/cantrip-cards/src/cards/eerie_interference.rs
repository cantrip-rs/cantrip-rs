use cantrip_api::*;
use cantrip_mechanics::prevention;
use cantrip_oracle as oracle;

/// "Prevent all damage that would be dealt to you and creatures you control this turn by creatures" (CR 615.1, 611.2a): damage
/// of any kind (combat or not) whose source is a creature (`prevention::damage_by_a_creature`), to its controller, or to a creature that player controls when it is
/// dealt (not to a planeswalker, a battle or another player, and not damage from a noncreature source). A creature that has left
/// the battlefield is still a creature source (its last known information, CR 608.2h), a creature card in a graveyard is not; it is the damage that is prevented, so
/// the effect is not about any one object (CR 615.3) and a creature that comes under your control later this turn is covered.
fn creature_damage_to_you_and_creatures_you_control(q: &dyn Query, asker: &Asker, _: ObjectId, d: &DamageEvent) -> bool {
    prevention::damage_by_a_creature(q, d)
        && (d.target == Target::Player(asker.controller)
            || d.target.object().is_some_and(|o| q.chars(o).is(CardType::Creature) && q.controller(o) == asker.controller))
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[], |g, item| {
        prevention::this_turn(g, item, Target::None, 0, Shield::All);
        Ok(())
    }),
    granted_preventions: &[GrantedPreventionDef {
        hint: "prevents all damage creatures would deal to you and creatures you control",
        applies: creature_damage_to_you_and_creatures_you_control,
        ..prevention::ANY_DAMAGE.text("Prevent all damage that would be dealt to you and creatures you control this turn by creatures.")
    }],
    ..oracle::EERIE_INTERFERENCE
};
