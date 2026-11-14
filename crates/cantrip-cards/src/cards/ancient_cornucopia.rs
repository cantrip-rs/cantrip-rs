use cantrip_api::*;
use cantrip_mechanics::effects::ask_may;
use cantrip_mechanics::triggers;
use cantrip_oracle as oracle;

const ASK: Question = Question::new("GainLife", "gain 1 life for each of that spell's colors?", "One life for each color of the spell you cast  ·  only once each turn", "Gain life", "Don't gain life");

/// The triggered ability's place among the card's abilities (the mana ability is the first).
fn gain_ability(q: &dyn Query, source: ObjectId) -> AbilityRef {
    AbilityRef { card: q.chars(source).card, index: 1 }
}

/// "Whenever you cast a spell that's one or more colors" (CR 603.2, 601.2i): a spell its controller cast, judged on the
/// spell as it is on the stack, with at least one color. Not at all once its controller has gained life with it this turn
/// (CR 603.2h).
fn you_cast_a_colored_spell(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    if q.history().done(me.source, gain_ability(q, me.source), me.controller) {
        return None;
    }
    triggers::you_cast_a_spell_counting_its_colors(q, me, event).filter(|fired| fired.data[0] > 0)
}

/// "You may gain 1 life for each of that spell's colors. Do this only once each turn." Asked only while its controller
/// has not gained life with this permanent this turn, and spent only by a yes: declining leaves the next colored spell
/// free to trigger it. The colors are the spell's as this resolves, or as it last was if it has left the stack (CR 608.2h).
fn may_gain_life(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if g.history().done(item.object, item.ability, item.controller) {
        return Ok(());
    }
    if !ask_may(g, item.controller, item.object, Purpose::Ask(&ASK))? {
        return Ok(());
    }
    g.note_done(item.object, item.ability, item.controller);
    let colors = triggers::colors_of_that_spell(g, item);
    g.gain_life(item.controller, colors as i32);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        mana_ability(TAP, add_any_color(1), "{T}: Add one mana of any color."),
        triggered(
            when_asked(EventMask::SPELL_CAST, you_cast_a_colored_spell),
            may_gain_life,
            "Whenever you cast a spell that's one or more colors, you may gain 1 life for each of that spell's colors. Do this only once each turn.",
        ),
    ],
    ..oracle::ANCIENT_CORNUCOPIA
};
