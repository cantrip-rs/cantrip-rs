use cantrip_api::*;
use cantrip_mechanics::targets;
use cantrip_mechanics::triggers::another_creature_you_control_enters;
use cantrip_oracle as oracle;

/// A creature's power and toughness: as it has them now on the battlefield, or as it last had them there once it has left (CR 608.2h,
/// 113.7a); none for one with neither.
fn power_and_toughness(q: &dyn Query, id: ObjectId) -> (i16, i16) {
    if targets::permanent(q, id) {
        let chars = q.chars(id);
        (chars.power.unwrap_or(0), chars.toughness.unwrap_or(0))
    } else {
        q.state().last_known(id).map_or((0, 0), |known| (known.power.unwrap_or(0), known.toughness.unwrap_or(0)))
    }
}

/// "If it has greater power or toughness than Hulkling" (CR 603.4): the entering creature has the greater power, or the greater
/// toughness, of the two - not both needed.
fn greater_power_or_toughness(q: &dyn Query, creature: ObjectId, hulkling: ObjectId) -> bool {
    let ((power, toughness), (his_power, his_toughness)) = (power_and_toughness(q, creature), power_and_toughness(q, hulkling));
    power > his_power || toughness > his_toughness
}

/// "Whenever another creature you control enters, if it has greater power or toughness than Hulkling" (CR 603.2, 603.4): the
/// trigger condition, asked of the creature as it enters.
fn bigger_creature_enters(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    let fired = another_creature_you_control_enters(q, me, event)?;
    match fired.subject {
        Target::Object(creature) if greater_power_or_toughness(q, creature, me.source) => Some(fired),
        _ => None,
    }
}

/// "... put a +1/+1 counter on Hulkling." The intervening "if" is true again as the ability resolves (CR 603.4): the creature
/// that entered is still bigger than Hulkling as each last was, or else it does nothing.
fn put_counter(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Target::Object(creature) = item.subject
        && greater_power_or_toughness(g, creature, item.object)
    {
        g.add_counters(item.object, CounterKind::PlusOne, 1);
    }
    Ok(())
}

/// Vigilance is a core keyword.
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        when_asked(EventMask::ZONE_CHANGE, bigger_creature_enters),
        put_counter,
        "Whenever another creature you control enters, if it has greater power or toughness than Hulkling, put a +1/+1 counter on Hulkling.",
    )],
    ..oracle::HULKLING_BURGEONING_BRUISER
};
