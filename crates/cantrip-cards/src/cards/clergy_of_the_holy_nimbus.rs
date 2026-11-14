use cantrip_api::*;
use cantrip_oracle as oracle;

/// "If this creature would be destroyed, regenerate it." (CR 701.19b).
fn if_creature_would_be_destroyed_applies(
    _: &dyn Query,
    asker: &Asker,
    proposal: &Proposal,
) -> bool {
    let Proposal::ZoneChange(z) = proposal else { return false };
    let id = z.object;
    if z.destruction() && asker.source == id {
        return true;
    }
    false
}

/// "If this creature would be destroyed, regenerate it." (CR 701.19b).
fn if_creature_would_be_destroyed_apply(
    _: &dyn Query,
    _: &Asker,
    proposal: &mut Proposal,
) {
    let seen = *proposal;
    let Proposal::ZoneChange(_) = &seen else { return };
    let Proposal::ZoneChange(z) = proposal else { return };
    z.regenerated = z.destroy;
}

/// "{1}: This creature can't be regenerated this turn. Only your opponents may activate this ability." (CR 608.2, 611.2a).
fn creature_cant_be_regenerated_only(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let objects_0 = Some(item.object).into_iter().collect::<Vec<ObjectId>>();
    instruction(
        g,
        |g| {
            for &object in &objects_0 {
                g.add_effect(
                    item.object,
                    object,
                    EffectKind::CantBeRegenerated,
                    Duration::EndOfTurn,
                );
            }
        },
    );
    Ok(())
}

/// "{1}: This creature can't be regenerated this turn. Only your opponents may activate this ability." (CR 602.2).
fn creature_cant_be_regenerated_only_activators(
    q: &dyn Query,
    asker: &Asker,
    player: PlayerId,
) -> bool {
    q.state().opponents(asker.controller).collect::<Vec<PlayerId>>().contains(&player)
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(
                cost("{1}"),
                creature_cant_be_regenerated_only,
                "{1}: This creature can't be regenerated this turn. Only your opponents may activate this ability.",
            )
            .activated_by(creature_cant_be_regenerated_only_activators),
    ],
    replacements: &[
        replacement(
            ReplacementKind::ZoneChange,
            if_creature_would_be_destroyed_applies,
            if_creature_would_be_destroyed_apply,
            "If this creature would be destroyed, regenerate it.",
        ),
    ],
    ..oracle::CLERGY_OF_THE_HOLY_NIMBUS
};
