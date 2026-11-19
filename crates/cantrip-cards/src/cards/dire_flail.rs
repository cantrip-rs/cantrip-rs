use cantrip_api::*;
use cantrip_mechanics::attach::{attached_object, equip};
use cantrip_mechanics::statics::{grant_delayed_ability, plus_pt};
use cantrip_mechanics::{choose, craft, damage_target, targets};
use cantrip_oracle as oracle;

/// "Whenever this creature attacks" (CR 508.1, 603.2), of the ability a Dire Blunderbuss gave the creature (CR 613.1f): the
/// creature with it, declared as an attacker. "Dire Blunderbuss" in its text is the Blunderbuss that gave this instance of
/// it, so each of several Blunderbusses on one creature gives an ability naming itself: the Blunderbuss is the ability's
/// giver as the layers have it (`Query::ability_giver`), carried as the trigger's subject, so it is the same Blunderbuss as
/// the ability resolves.
fn this_creature_attacks(q: &dyn Query, asker: &Asker, event: &Event) -> Option<Fired> {
    let Event::Attacks { attacker, .. } = event else { return None };
    if *attacker != asker.source {
        return None;
    }
    Fired::about(Target::Object(q.ability_giver(asker)?))
}

/// "You may sacrifice an artifact other than Dire Blunderbuss. When you do, this creature deals damage equal to its power
/// to target creature." Declining is the empty answer to an up-to-one choice; the reflexive ability (CR 603.12) exists
/// only if an artifact was sacrificed, and its target is chosen as it goes on the stack (the card's ruling).
fn may_sacrifice(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let you = item.controller;
    let blunderbuss = item.subject.object();
    let artifacts: Vec<ObjectId> =
        g.permanents_of(you).into_iter().filter(|&id| Some(id) != blunderbuss && g.chars(id).is(CardType::Artifact)).collect();
    let chosen = choose::cards(g, you, item.object, Purpose::Sacrifice, artifacts, 1, true)?;
    let Some(&artifact) = chosen.first() else { return Ok(()) };
    // A permanent of your own is always yours to sacrifice to your own ability (CR 701.21a): chosen, it is sacrificed.
    g.sacrifice(you, &[artifact]);
    g.reflexive_trigger(item.object, you, AbilityRef::delayed(item.ability.card, 1), Target::None);
    Ok(())
}

/// "This creature deals damage equal to its power to target creature": its power as the ability resolves, or as it last
/// existed on the battlefield if it has left (the card's rulings, CR 608.2h); the creature is the source, so its lifelink
/// counts as it last was.
fn deal_power(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let power = g.chars(item.object).power.unwrap_or(0).max(0) as u32;
    damage_target(g, item, 0, power);
    Ok(())
}

const ATTACK_TEXT: &str = "Whenever this creature attacks, you may sacrifice an artifact other than Dire Blunderbuss. When you do, this creature deals damage equal to its power to target creature.";

pub static BACK: CardDef = CardDef {
    abilities: &[equip(mana("{1}"), "Equip {1}")],
    delayed: &[
        triggered(when_asked(EventMask::ATTACKS, this_creature_attacks), may_sacrifice, ATTACK_TEXT),
        delayed_trigger(deal_power, "When you do, this creature deals damage equal to its power to target creature.").targets(&[targets::CREATURE]),
    ],
    statics: &[
        static_ability(Layer::PtModify, attached_object, plus_pt::<3, 0>, "Equipped creature gets +3/+0."),
        static_ability(Layer::Ability, attached_object, grant_delayed_ability, "Equipped creature has \"Whenever this creature attacks, you may sacrifice an artifact other than Dire Blunderbuss. When you do, this creature deals damage equal to its power to target creature.\""),
    ],
    ..oracle::DIRE_BLUNDERBUSS
};

pub static CARD: CardDef = CardDef {
    abilities: &[equip(mana("{1}"), "Equip {1}"), craft::ability(mana("{3}{R}{R}"), &craft::ARTIFACT, "Craft with artifact {3}{R}{R}")],
    statics: &[static_ability(Layer::PtModify, attached_object, plus_pt::<2, 0>, "Equipped creature gets +2/+0.")],
    back: Some(&BACK),
    ..oracle::DIRE_FLAIL
};
