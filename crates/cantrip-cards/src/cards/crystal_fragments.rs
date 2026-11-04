use cantrip_api::*;
use cantrip_mechanics::attach::{attached_object, equip};
use cantrip_mechanics::statics::plus_pt;
use cantrip_mechanics::{effects, exile_and_return_transformed, prevention};
use cantrip_oracle as oracle;

/// "{5}{W}{W}: Exile this Equipment, then return it to the battlefield transformed under its owner's control" (CR 712.14a):
/// nothing happens if it has left the battlefield already (CR 400.7).
fn transform(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let owner = g.obj(item.object).owner;
    exile_and_return_transformed(g, item.object, owner)
}

/// Damage of any kind to the creatures the effect's controller controls (CR 615.1, 120.1): it is read as the damage would be
/// dealt, so a creature that arrives later in the turn is covered and one that has changed hands is not.
fn dealt_to_creature_you_control(q: &dyn Query, asker: &Asker, _: ObjectId, d: &DamageEvent) -> bool {
    matches!(d.target, Target::Object(id) if q.is_on_battlefield(id) && q.chars(id).is(CardType::Creature) && q.controller(id) == asker.controller)
}

const DAMAGE_TO_CREATURES_YOU_CONTROL: GrantedPreventionDef = GrantedPreventionDef {
    hint: "prevents all damage dealt to creatures you control",
    damage: "damage",
    of_it: None,
    applies: dealt_to_creature_you_control,
    then: None,
    text: "Prevent all damage that would be dealt to creatures you control this turn.",
};

/// I, II: "Prevent all damage that would be dealt to creatures you control this turn." (CR 615.1, 611.2a).
fn prevent_damage_to_your_creatures(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    prevention::this_turn(g, item, Target::None, 0, Shield::All);
    Ok(())
}

/// III: "Tap all creatures your opponents control." (CR 701.26a).
fn tap_opponents_creatures(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::tap_all_creatures_opponents_control(g, item);
    Ok(())
}

/// Summon: Alexander, the back face: a white 4/3 Saga Construct creature with flying (CR 714).
pub static BACK: CardDef = CardDef {
    abilities: &[
        chapter(1, prevent_damage_to_your_creatures, "I, II \u{2014} Prevent all damage that would be dealt to creatures you control this turn."),
        chapter(2, prevent_damage_to_your_creatures, "I, II \u{2014} Prevent all damage that would be dealt to creatures you control this turn."),
        chapter(3, tap_opponents_creatures, "III \u{2014} Tap all creatures your opponents control."),
    ],
    granted_preventions: &[DAMAGE_TO_CREATURES_YOU_CONTROL],
    ..oracle::SUMMON_ALEXANDER
};

pub static CARD: CardDef = CardDef {
    abilities: &[
        equip(mana("{1}"), "Equip {1}"),
        ability(cost("{5}{W}{W}"), transform, "{5}{W}{W}: Exile this Equipment, then return it to the battlefield transformed under its owner's control. Activate only as a sorcery.")
            .sorcery_speed(),
    ],
    statics: &[static_ability(Layer::PtModify, attached_object, plus_pt::<1, 1>, "Equipped creature gets +1/+1.")],
    back: Some(&BACK),
    ..oracle::CRYSTAL_FRAGMENTS
};
