use cantrip_api::*;
use cantrip_mechanics::discard_chosen;
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

/// "Whenever this creature deals damage to a player, that player discards a card." (CR 608.2, 701.9a).
fn player_discards_card(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    discard_chosen(g, PlayerId(item.data[1] as u8), item.object, 1)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            when_asked(EventMask::DAMAGE, when_creature_deals_damage_player),
            player_discards_card,
            "Whenever this creature deals damage to a player, that player discards a card.",
        ),
    ],
    ..oracle::ABYSSAL_SPECTER
};
