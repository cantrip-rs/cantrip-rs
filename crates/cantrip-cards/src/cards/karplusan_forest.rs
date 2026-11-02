use cantrip_api::*;
use cantrip_oracle as oracle;

/// "{T}: Add {C}." (CR 605.3a, 106.3).
fn add_c_mana(g: &mut dyn Game, asker: &Asker, _: Option<Color>) -> Result<(), Suspend> {
    g.add_mana(asker.controller, Mana::C, 1);
    Ok(())
}

/// "{T}: Add {R} or {G}. This land deals 1 damage to you." (CR 605.3a, 120.3, 106.3).
fn add_r_g_land_deals_mana(
    g: &mut dyn Game,
    asker: &Asker,
    chosen: Option<Color>,
) -> Result<(), Suspend> {
    {
        let Some(color) = chosen else { return Ok(()) };
        g.add_mana(asker.controller, color.mana(), 1);
    }
    let recipients_2 = Some(asker.controller)
        .into_iter()
        .map(Target::Player)
        .collect::<Vec<Target>>();
    let batch: Vec<(ObjectId, Target, u16)> = recipients_2
        .iter()
        .map(|t| (asker.source, *t, 1u16))
        .collect();
    g.damage_together(&batch);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        mana_ability(
            TAP,
            Adds {
                pool: ManaPool([0, 0, 0, 0, 0, 1]),
                ..add_by(add_c_mana)
            },
            "{T}: Add {C}.",
        ),
        mana_ability(
            TAP,
            add_by(add_r_g_land_deals_mana)
                .choose_color()
                .colors(ColorSet(24))
                .yields_any(1),
            "{T}: Add {R} or {G}. This land deals 1 damage to you.",
        ),
    ],
    ..oracle::KARPLUSAN_FOREST
};
