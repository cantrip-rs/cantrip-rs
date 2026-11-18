use cantrip_api::*;
use cantrip_mechanics::select::{self, Selection};
use cantrip_mechanics::{choose, effects, targets};
use cantrip_oracle as oracle;

/// Hustle: "Target creature attacks or blocks this turn if able." Two requirements (CR 508.1d, 509.1c) on the one creature,
/// each of which applies only in the step it concerns. Its one target illegal, the spell does not resolve (CR 608.2b).
fn hustle(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    instruction(g, |g| {
        g.add_effect(item.object, creature, EffectKind::MustAttack, Duration::EndOfTurn);
        g.add_effect(item.object, creature, EffectKind::MustBlock, Duration::EndOfTurn);
    });
    Ok(())
}

/// Bustle: "Creatures you control get +2/+2 and gain trample until end of turn. You may turn a creature you control face up."
/// Only a face-down creature can be turned face up, so those are the ones offered; the pump is already done by then, so the
/// creature turned up has it (the effect is on the permanent, and turning face up does not make it a new object, CR 708.8).
fn bustle(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::pump_and_trample_creatures_you_control(g, item, 2, 2);
    let face_down: Vec<ObjectId> = effects::creatures_on_the_battlefield(g)
        .into_iter()
        .filter(|&id| g.controller(id) == item.controller && g.obj(id).face_down)
        .collect();
    let chosen = choose::select(g, item.controller, item.object, Purpose::Choose, face_down, Selection::of(select::up_to(1), &[]).called("face-down creature"))?;
    for permanent in chosen {
        g.turn_face_up(permanent, item.controller);
    }
    Ok(())
}

pub static HUSTLE: CardDef = CardDef { spell: spell(&[targets::CREATURE], hustle), ..oracle::HUSTLE };

pub static BUSTLE: CardDef = CardDef { spell: spell(&[], bustle), ..oracle::BUSTLE };

pub static CARD: CardDef = CardDef { split: Some((&HUSTLE, &BUSTLE)), ..oracle::HUSTLE_BUSTLE };

pub static EXTRA: &[&CardDef] = &[&HUSTLE, &BUSTLE];
