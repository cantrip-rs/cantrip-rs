use cantrip_api::*;
use cantrip_mechanics::triggers;
use cantrip_oracle as oracle;

/// "Whenever Cecil deals damage" (CR 120.2), to anything: one trigger for the damage it deals at once, carrying that
/// much (`sums_per_subject` adds up the events of a combat that hurt several blockers).
fn cecil_deals_damage(_: &dyn Query, source: ObjectId, event: &Event) -> Option<Fired> {
    match event {
        Event::Damage { source: from, amount, .. } if *from == source && *amount > 0 => {
            Some(Fired { subject: Target::None, data: [0, 0] })
        }
        _ => None,
    }
}

/// "You lose that much life. Then if your life total is less than or equal to half your starting life total, untap
/// Cecil and transform it."
fn darkness(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let you = item.controller;
    g.lose_life(you, item.matched_damage_amount() as i32);
    let player = *g.state().player(you);
    if player.life as i64 * 2 <= player.starting_life as i64 {
        g.untap(item.object);
        g.transform_by(item.object, item)?;
    }
    Ok(())
}

/// "Other attacking creatures gain indestructible until end of turn."
fn protect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let others: Vec<ObjectId> = g
        .state()
        .attackers
        .iter()
        .map(|a| a.id)
        .filter(|&id| id != item.object && g.is_attacking(id) && g.is_on_battlefield(id) && g.chars(id).is(CardType::Creature))
        .collect();
    instruction(g, |g| {
        for id in others {
            cantrip_mechanics::effects::gain_indestructible_until_end_of_turn(g, item.object, id);
        }
    });
    Ok(())
}

pub static BACK: CardDef = CardDef {
    abilities: &[triggered(triggers::THIS_ATTACKS, protect, "Protect \u{2014} Whenever Cecil attacks, other attacking creatures gain indestructible until end of turn.")],
    ..oracle::CECIL_REDEEMED_PALADIN
};

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(when(EventMask::DAMAGE, cecil_deals_damage), darkness, "Darkness \u{2014} Whenever Cecil deals damage, you lose that much life. Then if your life total is less than or equal to half your starting life total, untap Cecil and transform it.").damage_amount()
        .sums_per_subject()],
    back: Some(&BACK),
    ..oracle::CECIL_DARK_KNIGHT
};
