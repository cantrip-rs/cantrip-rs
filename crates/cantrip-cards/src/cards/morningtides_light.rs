use cantrip_api::*;
use cantrip_mechanics::{delayed, library, prevention, targets};
use cantrip_oracle as oracle;

/// "At the beginning of the next end step, return those cards to the battlefield tapped under their owners' control."
/// (CR 603.7): the cards the delayed ability is about that are still the cards they were and still in exile (CR 603.7c,
/// 400.7; a token has ceased to exist, CR 111.7) enter together (CR 101.4) tapped (CR 110.2a), each under its owner's
/// control.
fn return_tapped(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let cards = library::still_exiled(g, g.event_objects(item));
    let entering: Vec<(ObjectId, PlayerId, bool)> = cards.into_iter().map(|c| (c, g.obj(c).owner, true)).collect();
    g.put_all_onto_battlefield_as(&entering)?;
    Ok(())
}

/// "Exile any number of target creatures." (CR 608.2b, 406.2): those still legal are exiled together (CR 101.4), and
/// "at the beginning of the next end step, return those cards" is a delayed triggered ability about them. "Until your next
/// turn, prevent all damage that would be dealt to you." (CR 615.1, 611.2a): a prevention effect about the player, which
/// outlives the spell. "Exile Morningtide's Light." (CR 608.2c): its last instruction, so it goes to exile rather than to
/// its owner's graveyard.
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let you = item.controller;
    let exiled = g.exile_all(&targets::legal_objects(g, item));
    delayed::at_next_end_step_about(g, item, 0, &exiled);
    g.end_sentence();
    instruction(g, |g| {
        let shield = EffectKind::Prevention { card: item.ability.card, index: 0, to: Target::Player(you), shield: Shield::All };
        g.add_player_effect(you, item.object, shield, Duration::UntilNextTurnOf(you));
    });
    g.end_sentence();
    g.exile(item.object);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE.between(0, TargetSpec::ANY_NUMBER)], effect),
    delayed: &[delayed_trigger(return_tapped, "Return those cards to the battlefield tapped under their owners' control.")],
    granted_preventions: &[prevention::ANY_DAMAGE],
    ..oracle::MORNINGTIDES_LIGHT
};
