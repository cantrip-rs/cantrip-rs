use cantrip_api::*;
use cantrip_mechanics::{attach::equip, targets, triggers::THIS_ENTERS};
use cantrip_oracle as oracle;

/// "Put a hone counter on Sting for each creature target opponent controls. Attach Sting to up to one target creature you
/// control." The creatures are counted as the ability resolves; a target that has become illegal does nothing, and the
/// other part still happens (CR 608.2b). Sting gone from the battlefield gets no counters and attaches to nothing (CR
/// 400.7, 701.3b): the verbs see to both. Each hone counter gives the equipped creature +1/+0 (CR 122.1j, the layers).
fn hone_and_attach(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    // None for an opponent who is no longer a legal target, or who controls no creature: then no counter is put.
    let creatures = item.target(0).player().filter(|_| g.target_legal(item, 0)).map_or(0, |opponent| {
        g.permanents_of(opponent).into_iter().filter(|&id| g.chars(id).is(CardType::Creature)).count()
    });
    g.add_counters(item.object, CounterKind::Hone, i64::from(creatures.min(i16::MAX as usize) as i16));
    if let Some(creature) = targets::object(g, item, 1) {
        g.attach(item.object, creature);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(THIS_ENTERS, hone_and_attach, "When Sting enters, put a hone counter on Sting for each creature target opponent controls. Attach Sting to up to one target creature you control. (Each hone counter on an Equipment grants +1/+0 to equipped creature.)")
            .targets(&[
                targets::OPPONENT.labelled("Put a hone counter on Sting for each creature target opponent controls"),
                targets::CREATURE_YOU_CONTROL.optional().labelled("Attach Sting to up to one target creature you control"),
            ]),
        equip(mana("{3}"), "Equip {3}"),
    ],
    ..oracle::STING_BILBOS_SWORD
};
