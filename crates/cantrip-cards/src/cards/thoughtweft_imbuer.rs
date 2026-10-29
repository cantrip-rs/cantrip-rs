use cantrip_api::*;
use cantrip_mechanics::triggers::CREATURE_YOU_CONTROL_ATTACKS_ALONE;
use cantrip_oracle as oracle;

/// "It gets +X/+X until end of turn, where X is the number of Kithkin you control" (CR 506.5, 611.2a, 608.2h): the creature that
/// attacked alone, whichever creature of its controller's it is. X is worked out once, as the ability resolves (the ruling), the
/// Kithkin read through the words of this card's text (CR 612); the attacker is one of them if it is a Kithkin.
fn pump_by_kithkin(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Target::Object(creature) = item.subject else { return Ok(()) };
    let kithkin = g.words(item.object).subtype(Subtype::Kithkin);
    let x = g
        .state()
        .battlefield()
        .iter()
        .filter(|&&id| g.controller(id) == item.controller && g.chars(id).has_subtype(kithkin))
        .count() as i16;
    cantrip_mechanics::pump(g, item.object, creature, x, x);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        CREATURE_YOU_CONTROL_ATTACKS_ALONE,
        pump_by_kithkin,
        "Whenever a creature you control attacks alone, it gets +X/+X until end of turn, where X is the number of Kithkin you control.",
    )],
    ..oracle::THOUGHTWEFT_IMBUER
};
