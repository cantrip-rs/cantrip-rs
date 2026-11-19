use cantrip_api::*;
use cantrip_mechanics::{effects, targets, tokens};
use cantrip_oracle as oracle;

/// "Draw a card. Create a Sorcerer Role token attached to up to one other target creature you control." The draw and the
/// Role are one ability's two instructions (CR 608.2c). Choosing no target is legal (CR 115.1b), so the card is drawn and no
/// Role is created; a chosen target that is no longer legal when the ability resolves leaves it with no legal target, so
/// it does nothing at all and draws no card (CR 608.2b; the resolver fizzles it before this runs). A Role that could not
/// enchant the creature is not created (CR 303.4i).
fn draw_then_create_a_sorcerer_role(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::draw(g, item, 1)?;
    g.end_sentence();
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    tokens::create_role(g, &tokens::SORCERER_ROLE, item.controller, creature)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        cost("{1}").and(SACRIFICE_THIS),
        draw_then_create_a_sorcerer_role,
        "{1}, Sacrifice this creature: Draw a card. Create a Sorcerer Role token attached to up to one other target creature you control. Activate only as a sorcery.",
    )
    .targets(&[up_to_one(targets::other_creature_you_control_filter)])
    .sorcery_speed()],
    ..oracle::LIVING_LECTERN
};

pub static EXTRA: &[&CardDef] = &[&tokens::SORCERER_ROLE];
