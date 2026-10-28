use cantrip_api::*;
use cantrip_mechanics::attach::{attached_object, equip};
use cantrip_mechanics::statics::plus_pt;
use cantrip_mechanics::{tokens, triggers};
use cantrip_oracle as oracle;

/// The colorless Equipment artifact token named Sturdy Shield, "Equipped creature gets +1/+2" and equip {2}: the token's
/// name is "Sturdy Shield" (CR 111.4), it has no mana cost or color, and its subtype is Equipment (CR 301.5).
pub static STURDY_SHIELD: CardDef = CardDef {
    name: "Sturdy Shield",
    has_cost: false,
    types: TypeSet::of(&[CardType::Artifact]),
    subtypes: SubtypeSet::of(&[Subtype::Equipment]),
    abilities: &[equip(mana("{2}"), "Equip {2}")],
    statics: &[static_ability(Layer::PtModify, attached_object, plus_pt::<1, 2>, "Equipped creature gets +1/+2.")],
    text: "Equipped creature gets +1/+2.\nEquip {2}",
    ..CardDef::BLANK
};

/// "Create a ... Sturdy Shield ... Attach it to U.S.Agent": one resolution, so the token is attached (CR 701.3b) to him if
/// both are still on the battlefield.
fn create_and_attach_shield(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let made = tokens::create(g, item.controller, &STURDY_SHIELD, 1)?;
    let Some(&shield) = made.first() else { return Ok(()) };
    if g.is_on_battlefield(item.object) {
        g.attach(shield, item.object);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::THIS_ENTERS,
        create_and_attach_shield,
        "When U.S.Agent enters, create a colorless Equipment artifact token named Sturdy Shield with \"Equipped creature gets +1/+2\" and equip {2}. Attach it to U.S.Agent.",
    )],
    ..oracle::U_S_AGENT_JOHN_WALKER
};

pub static EXTRA: &[&CardDef] = &[&STURDY_SHIELD];
