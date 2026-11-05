use cantrip_api::*;
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

/// "Put a decayed counter on each of X target creatures" (CR 122.1b): each one still a legal target (CR 608.2b).
fn renew(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let puts: Vec<(ObjectId, CounterKind, i64)> =
        targets::legal_objects(g, item).into_iter().map(|creature| (creature, CounterKind::Decayed, 1)).collect();
    g.add_counters_together(&puts);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    // Trample and decayed are core keywords (CR 702.19, 702.147a), read from the card's JSON. Renew is an ability word
    // (CR 207.2c); X target creatures: the slot is counted by the X announced, and asked once it is (CR 601.2b-c).
    abilities: &[ability(cost("{X}{B}{B}").and(EXILE_THIS), renew, "Renew \u{2014} {X}{B}{B}, Exile this card from your graveyard: Put a decayed counter on each of X target creatures. Activate only as a sorcery.")
        .from(ZoneKind::Graveyard)
        .sorcery_speed()
        .targets(&[targets::CREATURE.counted()])],
    ..oracle::ROT_CURSE_RAKSHASA
};
