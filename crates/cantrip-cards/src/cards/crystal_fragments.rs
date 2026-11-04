use cantrip_api::*;
use cantrip_mechanics::attach::{attached_object, equip};
use cantrip_mechanics::statics::plus_pt;
use cantrip_mechanics::{exile_and_return_transformed, prevention};
use cantrip_oracle as oracle;

/// "{5}{W}{W}: Exile this Equipment, then return it to the battlefield transformed under its owner's control" (CR 712.14a):
/// nothing happens if it has left the battlefield already (CR 400.7).
fn transform(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let owner = g.obj(item.object).owner;
    exile_and_return_transformed(g, item.object, owner)
}

/// I, II: "Prevent all damage that would be dealt to creatures you control this turn." (CR 615.1, 611.2a).
fn prevent_damage_to_your_creatures(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    prevention::this_turn(g, item, Target::None, 0, Shield::All);
    Ok(())
}

/// III: "Tap all creatures your opponents control." (CR 701.26a): every creature a player other than the chapter's controller
/// controls, tapped at once.
fn tap_opponents_creatures(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let creatures: Vec<ObjectId> = g
        .state()
        .battlefield()
        .iter()
        .copied()
        .filter(|&id| g.chars(id).is(CardType::Creature) && g.controller(id) != item.controller)
        .collect();
    g.tap_together(&creatures);
    Ok(())
}

/// Summon: Alexander, the back face: a white 4/3 Saga Construct creature with flying (CR 714).
pub static BACK: CardDef = CardDef {
    abilities: &[
        chapter(1, prevent_damage_to_your_creatures, "I, II \u{2014} Prevent all damage that would be dealt to creatures you control this turn."),
        chapter(2, prevent_damage_to_your_creatures, "I, II \u{2014} Prevent all damage that would be dealt to creatures you control this turn."),
        chapter(3, tap_opponents_creatures, "III \u{2014} Tap all creatures your opponents control."),
    ],
    granted_preventions: &[prevention::DAMAGE_TO_CREATURES_YOU_CONTROL.text("Prevent all damage that would be dealt to creatures you control this turn.")],
    ..oracle::SUMMON_ALEXANDER
};

pub static CARD: CardDef = CardDef {
    abilities: &[
        equip(mana("{1}"), "Equip {1}"),
        ability(cost("{5}{W}{W}"), transform, "{5}{W}{W}: Exile this Equipment, then return it to the battlefield transformed under its owner's control. Activate only as a sorcery.")
            .timing(Timing::Sorcery),
    ],
    statics: &[static_ability(Layer::PtModify, attached_object, plus_pt::<1, 1>, "Equipped creature gets +1/+1.")],
    back: Some(&BACK),
    ..oracle::CRYSTAL_FRAGMENTS
};
