use cantrip_api::*;
use cantrip_oracle as oracle;

/// "Whenever this creature attacks and isn't blocked" (CR 603.2).
fn when_creature_attacks_isnt_blocked(
    _: &dyn Query,
    asker: &Asker,
    event: &Event,
) -> Option<Fired> {
    let Event::Unblocked { attacker, .. } = event else { return None };
    let id = *attacker;
    let subject = Target::Object(id);
    if asker.source == id {
        return Fired::about(subject);
    }
    None
}

/// "Whenever this creature attacks and isn't blocked, it gets +2/+0 until end of combat." (CR 608.2, 611.2a).
fn gets_2_0_combat(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let objects_0 = item.subject.object().into_iter().collect::<Vec<ObjectId>>();
    instruction(
        g,
        |g| {
            for &object in &objects_0 {
                g.add_effect(
                    item.object,
                    object,
                    EffectKind::Pump(2, 0),
                    Duration::EndOfCombat,
                );
            }
        },
    );
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            when_asked(EventMask::UNBLOCKED, when_creature_attacks_isnt_blocked),
            gets_2_0_combat,
            "Whenever this creature attacks and isn't blocked, it gets +2/+0 until end of combat.",
        ),
    ],
    ..oracle::MURK_DWELLERS
};
