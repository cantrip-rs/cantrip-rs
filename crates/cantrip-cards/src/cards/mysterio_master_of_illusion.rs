use cantrip_api::*;
use cantrip_mechanics::{lands, targets, tokens, triggers};
use cantrip_oracle as oracle;

/// The 3/3 blue Illusion Villain creature token Mysterio makes.
pub static ILLUSION_VILLAIN: CardDef = CardDef {
    colors: Some(ColorSet::of(Color::Blue)),
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(3),
    toughness: Some(3),
    subtypes: SubtypeSet::of(&[Subtype::Illusion, Subtype::Villain]),
    ..token!(Illusion Villain)
};

/// "Create a 3/3 blue Illusion Villain creature token for each nontoken Villain you control" (CR 111.1, 608.2c): counted as it
/// resolves, Mysterio himself among them while he is still there, the tokens made together. Then the delayed triggered ability
/// (CR 603.7) is set up, about Mysterio and carrying the tokens made (CR 603.7c): if he left the battlefield before this
/// resolved, it can never trigger (the 2025-09-19 ruling).
fn create_illusions(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let villain = lands::word(g, item.object, lands::sub(Subtype::Villain));
    let count = g
        .permanents_of(item.controller)
        .into_iter()
        .filter(|&id| !g.obj(id).token && g.chars(id).has_subtype(villain))
        .count() as u32;
    let made = tokens::create(g, item.controller, &ILLUSION_VILLAIN, count)?;
    g.delayed_trigger_about(DelayedWhen::WhenEvent, item.object, item.controller, AbilityRef::delayed(item.ability.card, 0), &made);
    Ok(())
}

/// "Exile those tokens": the ones made that are still on the battlefield, the same objects (CR 603.7c, 400.7).
fn exile_the_tokens(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let present: Vec<ObjectId> = g.event_objects(item).iter().copied().filter(|&id| targets::permanent(g, id)).collect();
    g.exile_all(&present);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::THIS_ENTERS,
        create_illusions,
        "When Mysterio enters, create a 3/3 blue Illusion Villain creature token for each nontoken Villain you control. Exile those tokens when Mysterio leaves the battlefield.",
    )],
    delayed: &[triggered(triggers::THIS_LEAVES, exile_the_tokens, "Exile those tokens when Mysterio leaves the battlefield.")],
    ..oracle::MYSTERIO_MASTER_OF_ILLUSION
};

pub static EXTRA: &[&CardDef] = &[&ILLUSION_VILLAIN];
