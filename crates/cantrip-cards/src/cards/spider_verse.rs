use cantrip_api::*;
use cantrip_mechanics::effects::ask_may;
use cantrip_mechanics::history::done_this_turn;
use cantrip_mechanics::targets::is_permanent_card;
use cantrip_mechanics::triggers;
use cantrip_oracle as oracle;

const ASK: Question = Question::new("CopySpell", "copy the spell?", "You may choose new targets for the copy  ·  only once each turn", "Copy it", "Don't");

/// "The 'legend rule' doesn't apply to Spiders you control" (CR 704.5j): a permanent with the creature type Spider, read through
/// this enchantment's own words (CR 612), under this enchantment's controller.
fn spider_you_control(q: &dyn Query, asker: &Asker, legend: ObjectId) -> bool {
    q.controller(legend) == asker.controller && q.chars(legend).has_subtype(q.words(asker.source).subtype(Subtype::Spider))
}

/// "Whenever you cast a spell from anywhere other than your hand" (CR 601.2, 603.2), but not at all once its controller has
/// chosen to copy a spell with this ability this turn (CR 603.2h).
fn you_cast_a_spell_from_outside_your_hand_once_each_turn(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    triggers::you_cast_a_spell_from_outside_your_hand(q, me, event).filter(|_| !done_this_turn(q, me))
}

/// "You may copy it. If you do, you may choose new targets for the copy. If the copy is a permanent spell, it gains haste. Do this
/// only once each turn." (CR 707.10, 707.10c): asked only while its controller has not copied a spell with this permanent this
/// turn (instances already on the stack when they do have no effect, the ruling), and spent only by a yes. A copy of a permanent
/// spell becomes a token as it resolves (CR 707.10f), keeping the haste it was given.
fn copy_it(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(spell) = item.subject.object() else { return Ok(()) };
    if g.history().done(item.object, item.ability, item.controller) || !ask_may(g, item.controller, item.object, Purpose::Ask(&ASK))? {
        return Ok(());
    }
    g.note_done(item.object, item.ability, item.controller);
    if let Some(copy) = g.copy_spell(spell, item.controller, true)?
        && is_permanent_card(&g.chars(copy))
    {
        cantrip_mechanics::effects::gain_haste(g, item.object, copy);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    legend_rule_exempt: &[spider_you_control],
    abilities: &[triggered(
        when_asked(EventMask::SPELL_CAST, you_cast_a_spell_from_outside_your_hand_once_each_turn),
        copy_it,
        "Whenever you cast a spell from anywhere other than your hand, you may copy it. If you do, you may choose new targets for the copy. If the copy is a permanent spell, it gains haste. Do this only once each turn.",
    )],
    ..oracle::SPIDER_VERSE
};
