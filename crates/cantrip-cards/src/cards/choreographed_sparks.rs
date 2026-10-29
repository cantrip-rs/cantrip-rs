use cantrip_api::*;
use cantrip_mechanics::{effects, targets, triggers};
use cantrip_oracle as oracle;

/// The ability the creature spell's copy gains (CR 707.10, 707.9a): "At the beginning of the end step, sacrifice this token."
/// Not a card anyone plays: a definition for the copy's gained ability to be referred to by. It is part of the copy's
/// copiable values, so it is still there on the token the copy becomes as it resolves (CR 707.10f).
pub static GAINED: CardDef = CardDef {
    name: "Choreographed Sparks (copy ability)",
    abilities: &[triggered(triggers::EACH_END_STEP, effects::sacrifice_self, "At the beginning of the end step, sacrifice this token.")],
    ..CardDef::BLANK
};

fn creature_spell_you_control(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    (targets::CREATURE_SPELL.filter)(q, asker, t) && matches!(t, Target::Object(id) if q.controller(id) == asker.controller)
}

/// Mode 1: "Copy target instant or sorcery spell you control. You may choose new targets for the copy" (CR 707.10, 707.10c). With
/// the target gone the mode does nothing (CR 608.2b).
fn copy_instant_or_sorcery(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(spell) = targets::object(g, item, 0) else { return Ok(()) };
    g.copy_spell(spell, item.controller, true)?;
    Ok(())
}

/// Mode 2: "Copy target creature spell you control. The copy gains haste and [the gained ability]" (CR 707.10, 707.9a): gained
/// by the copy as it is made, so the token it becomes has them.
fn copy_creature(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(spell) = targets::object(g, item, 0) else { return Ok(()) };
    let gained = AbilityRef { card: g.card_id(&GAINED), index: 0 };
    let values = g.copiable_values(spell).with_keyword(Keyword::Haste).with_ability(gained);
    g.copy_spell_as(spell, item.controller, false, values)?;
    Ok(())
}

/// "Choose one or both" (CR 700.2): the modes are carried out in the order printed (CR 608.2c).
static MODAL: ModalDef = modes(&[
    mode(copy_instant_or_sorcery, "Copy target instant or sorcery spell you control. You may choose new targets for the copy.")
        .targets(&[targets::INSTANT_OR_SORCERY_SPELL_YOU_CONTROL]),
    mode(
        copy_creature,
        "Copy target creature spell you control. The copy gains haste and \"At the beginning of the end step, sacrifice this token.\"",
    )
    .targets(&[target(creature_spell_you_control)]),
])
.choose(1, 2);

pub static CARD: CardDef = CardDef { spell: Some(SpellDef::MODAL), modal: Some(&MODAL), ..oracle::CHOREOGRAPHED_SPARKS };

pub static EXTRA: &[&CardDef] = &[&GAINED];
