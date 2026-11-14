use cantrip_api::*;
use cantrip_mechanics::{choose_color_for, effects};
use cantrip_oracle as oracle;

/// "As this creature enters, choose a color" (CR 614.12a): any of the five, kept on the permanent (CR 607.2d).
fn enters_choosing_a_color(g: &mut dyn Game, entering: &mut Entering) -> Result<(), Suspend> {
    entering.chosen_color = Some(choose_color_for(g, entering.controller, entering.object, ColorUse::Any)?);
    Ok(())
}

/// "Whenever you cast a spell of the chosen color" (CR 603.2): the spell's colors on the stack, the color chosen for the ability
/// that asks (`Query::linked_to`).
fn you_cast_a_spell_of_the_chosen_color(q: &dyn Query, asker: &Asker, event: &Event) -> Option<Fired> {
    match event {
        Event::SpellCast { spell, player, .. } if *player == asker.controller => {
            let chosen = q.state().chosen_color(q.linked_to(asker))?;
            q.chars(*spell).colors.has(chosen).then_some(Fired::default())
        }
        _ => None,
    }
}

pub static CARD: CardDef = CardDef {
    enters: Some(enters_choosing_a_color),
    abilities: &[triggered(
        when_asked(EventMask::SPELL_CAST, you_cast_a_spell_of_the_chosen_color),
        effects::gain_one_life,
        "Whenever you cast a spell of the chosen color, you gain 1 life.",
    )],
    ..oracle::DIAMOND_MARE
};
