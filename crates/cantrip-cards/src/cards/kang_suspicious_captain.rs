use cantrip_api::*;
use cantrip_oracle as oracle;
use cantrip_mechanics::{effects, goad, targets, triggers};

/// "When Kang enters, choose target creature an opponent controls. Kang and that creature are goaded for as long as you
/// control Kang." (CR 701.15, 611.2b): the chosen creature and Kang are goaded by Kang's controller until its controller
/// no longer controls Kang - the native source-control duration checks that - and each is goaded once, a creature
/// already goaded by that player gaining nothing more (CR 701.15d). Two separate sentences, each judged before the next
/// (CR 603.8, 608.2c).
fn goad_kang_and_the_chosen(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(chosen) = targets::object(g, item, 0) else { return Ok(()) };
    let until = Duration::WhileSourceControlled;
    goad::goad_for(g, item, chosen, until);
    g.end_sentence();
    goad::goad_for(g, item, item.object, until);
    Ok(())
}

/// "Whenever one or more goaded creatures attack" (CR 508.1, 603.2c): any goaded creature declared as an attacker, of
/// either player's - one trigger for the batch however many attack (CR 603.2c), which `once_per_batch` makes one.
fn goaded_creature_attacks(q: &dyn Query, _: &Asker, event: &Event) -> Option<Fired> {
    let Event::Attacks { attacker, .. } = event else { return None };
    if q.goaders(*attacker).is_empty() { None } else { Fired::PLAIN }
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            triggers::THIS_ENTERS,
            goad_kang_and_the_chosen,
            "When Kang enters, choose target creature an opponent controls. Kang and that creature are goaded for as long as you control Kang.",
        )
        .targets(&[targets::CREATURE_AN_OPPONENT_CONTROLS]),
        triggered(
            when_asked(EventMask::ATTACKS, goaded_creature_attacks),
            effects::plus_one_counter_on_target,
            "Whenever one or more goaded creatures attack, put a +1/+1 counter on target creature you control.",
        )
        .targets(&[targets::CREATURE_YOU_CONTROL])
        .once_per_batch(),
    ],
    ..oracle::KANG_SUSPICIOUS_CAPTAIN
};
