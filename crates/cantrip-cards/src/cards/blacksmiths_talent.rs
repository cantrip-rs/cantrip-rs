use cantrip_api::*;
use cantrip_mechanics::attach::{attach_first_target_to_second, attached_object, equip};
use cantrip_mechanics::statics::{equipped_creatures_you_control, have_double_strike_and_haste, plus_pt};
use cantrip_mechanics::triggers::{beginning_of_your_combat, THIS_ENTERS};
use cantrip_mechanics::{class, targets, tokens};
use cantrip_oracle as oracle;

/// The colorless Equipment artifact token named Sword, "Equipped creature gets +1/+1" and equip {2}: its name is "Sword" (CR 111.4), it
/// has no mana cost or color, and its subtype is Equipment (CR 301.5).
static SWORD: CardDef = CardDef {
    name: "Sword",
    has_cost: false,
    types: TypeSet::of(&[CardType::Artifact]),
    subtypes: SubtypeSet::of(&[Subtype::Equipment]),
    abilities: &[equip(mana("{2}"), "Equip {2}")],
    statics: &[static_ability(Layer::PtModify, attached_object, plus_pt::<1, 1>, "Equipped creature gets +1/+1.")],
    text: "Equipped creature gets +1/+1.\nEquip {2}",
    ..CardDef::BLANK
};

fn create_sword(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create(g, item.controller, &SWORD, 1)?;
    Ok(())
}

/// "At the beginning of combat on your turn", once the Class has reached level 2 (CR 716.2a, 603.2).
fn beginning_of_combat_at_level_two(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    if class::level(q, me.source) >= 2 { beginning_of_your_combat(q, me, event) } else { None }
}

/// "Equipped creatures you control", during your turn and only at level 3 (CR 716.2a, 604.1, 613.1f): read again whenever
/// characteristics are computed, so it stops at end of turn and when an Equipment is unattached.
fn equipped_creatures_you_control_during_your_turn_at_level_three(view: &dyn LayerQuery, source: ObjectId, affected: ObjectId, so_far: &Chars) -> bool {
    view.state().obj(source).class_level >= 3
        && view.state().turn.active == view.state().obj(source).controller
        && equipped_creatures_you_control(view, source, affected, so_far)
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            THIS_ENTERS,
            create_sword,
            "When this Class enters, create a colorless Equipment artifact token named Sword with \"Equipped creature gets +1/+1\" and equip {2}.",
        ),
        class::level_two("{2}{R}", "{2}{R}: Level 2"),
        triggered(
            when_asked(EventMask::STEP_BEGAN, beginning_of_combat_at_level_two),
            attach_first_target_to_second,
            "At the beginning of combat on your turn, attach target Equipment you control to up to one target creature you control.",
        )
        .targets(&[
            targets::EQUIPMENT_YOU_CONTROL.labelled("Equipment to attach"),
            up_to_one(targets::CREATURE_YOU_CONTROL.filter).labelled("Creature to attach it to"),
        ]),
        class::level_three("{3}{R}", "{3}{R}: Level 3"),
    ],
    statics: &[static_ability(
        Layer::Ability,
        equipped_creatures_you_control_during_your_turn_at_level_three,
        have_double_strike_and_haste,
        "During your turn, equipped creatures you control have double strike and haste.",
    )],
    ..oracle::BLACKSMITHS_TALENT
};

pub static EXTRA: &[&CardDef] = &[&SWORD];
