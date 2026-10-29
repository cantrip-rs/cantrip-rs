use cantrip_api::*;
use cantrip_mechanics::triggers::cast_this_spell;
use cantrip_mechanics::{creatures_you_control, each_player_sacrifices, effects};
use cantrip_oracle as oracle;

const ASK: Question = Question::new("CopySpell", "copy this spell?", "The copy is not cast", "Copy it", "Don't copy").no_default();

/// "When you cast this spell while you control a creature": the "while" is part of the trigger event (CR 603.2), judged as the
/// spell is cast and never again, so it is no intervening "if" (CR 603.4) and losing the creature before the ability resolves
/// changes nothing.
fn cast_while_you_control_a_creature(q: &dyn Query, source: ObjectId, event: &Event) -> Option<Fired> {
    let fired = cast_this_spell(q, source, event)?;
    (creatures_you_control(q, q.controller(source)) > 0).then_some(fired)
}

/// "You may copy this spell" (CR 707.10): the copy is not cast (CR 707.10), so nothing that triggers on a cast sees it.
fn may_copy(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if effects::ask_may(g, item.controller, item.object, Purpose::Ask(&ASK))? {
        g.copy_spell(item.object, item.controller, false)?;
    }
    Ok(())
}

/// "Each player sacrifices a creature of their choice. Each opponent loses 1 life and you gain 1 life." Every player chooses in
/// turn order and the creatures are sacrificed together (CR 101.4, 701.21a); a player with no creature sacrifices nothing.
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    each_player_sacrifices(g, item.object, 1, |q, id| q.chars(id).is(CardType::Creature))?;
    g.end_sentence();
    effects::each_opponent_loses_life_you_gain_life::<1>(g, item)
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[], effect),
    abilities: &[triggered(
        when_this_spell_is_cast(cast_while_you_control_a_creature),
        may_copy,
        "When you cast this spell while you control a creature, you may copy this spell.",
    )],
    ..oracle::SOCIAL_SNUB
};
