use cantrip_api::*;
use cantrip_mechanics::{
    attach::equip_quality,
    effects,
    targets,
    triggers::THIS_ENTERS,
};
use cantrip_oracle as oracle;

/// "Double all damage equipped creature would deal" (CR 614.1, 120.3): damage from the creature this Equipment is attached to,
/// to anything, combat or not; damage dealt to it, or by another creature, is not changed.
fn damage_from_equipped_creature(q: &dyn Query, asker: &Asker, p: &Proposal) -> bool {
    matches!(p, Proposal::Damage(d)
        if d.amount > 0 && q.attached_to(asker.source) == Some(d.source) && q.chars(d.source).is(CardType::Creature))
}

fn double(_: &dyn Query, _: &Asker, p: &mut Proposal) {
    if let Proposal::Damage(d) = p {
        d.amount = d.amount.checked_mul(2).expect("damage expression exceeds i64 semantic domain");
    }
}

/// "A creature is worthy if it's a legendary non-Villain that's red and/or white" (CR 700.16): a quality the rules define, so no
/// text-changing effect reaches its words.
fn worthy_creature_you_control(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    (targets::CREATURE_YOU_CONTROL.filter)(q, asker, t)
        && matches!(t, Target::Object(id) if {
            let c = q.chars(id);
            c.supertypes.legendary && !c.has_subtype(Subtype::Villain) && (c.colors.has(Color::Red) || c.colors.has(Color::White))
        })
}

/// "It deals 2 damage to each creature": the card discarded as a cost is the source (CR 113.7a, 608.2h); it is not the equipped
/// creature, so its damage is not doubled.
fn two_damage_to_each_creature(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::damage_each_creature(g, item, 2);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(THIS_ENTERS, effects::damage_first_target::<4>, "When Mjölnir enters, it deals 4 damage to up to one target creature.")
            .targets(&[targets::CREATURE.optional()]),
        equip_quality(mana("{1}"), &[target(worthy_creature_you_control)], "Equip worthy {1}"),
        ability(cost("{2}{R}").and(DISCARD_THIS), two_damage_to_each_creature, "{2}{R}, Discard this card: It deals 2 damage to each creature.")
            .from(ZoneKind::Hand),
    ],
    replacements: &[replacement(ReplacementKind::Damage, damage_from_equipped_creature, double, "double all damage equipped creature would deal")],
    ..oracle::MJ_LNIR_HAMMER_OF_THOR
};
