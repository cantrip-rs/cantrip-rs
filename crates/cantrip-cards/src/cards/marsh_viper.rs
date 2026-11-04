use cantrip_api::*;
use cantrip_oracle as oracle;

/// "Whenever this creature deals damage to a player" (CR 603.2).
fn when_creature_deals_damage_player(
    q: &dyn Query,
    asker: &Asker,
    event: &Event,
) -> Option<Fired> {
    let Event::Damage { source, target, .. } = event else { return None };
    let source = *source;
    let subject = Target::Object(source);
    let data = [
        0,
        match target {
            Target::Player(p) => u32::from(p.0),
            _ => u32::MAX,
        },
    ];
    if !q.state().obj(source).zone.is(ZoneKind::Stack) && asker.source == source
        && matches!(* target, Target::Player(_))
    {
        return Some(Fired { subject, data });
    }
    None
}

/// "Whenever this creature deals damage to a player, that player gets two poison counters." (CR 608.2).
fn player_gets_two_poison_counters(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    g.give_poison(PlayerId(item.data[1] as u8), 2);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            when_asked(EventMask::DAMAGE, when_creature_deals_damage_player),
            player_gets_two_poison_counters,
            "Whenever this creature deals damage to a player, that player gets two poison counters.",
        ),
    ],
    ..oracle::MARSH_VIPER
};
