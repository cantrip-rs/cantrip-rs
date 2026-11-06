use cantrip_api::*;
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

/// "Whenever enchanted creature deals damage to you" (CR 603.2).
fn when_enchanted_creature_deals_damage(
    q: &dyn Query,
    asker: &Asker,
    event: &Event,
) -> Option<Fired> {
    let Event::Damage { source, target, .. } = event else { return None };
    let source = *source;
    let subject = Target::Object(source);
    let data = [0, 0];
    if q.attached_to(asker.source) == Some(source)
        && matches!(* target, Target::Player(p) if asker.controller == p)
    {
        return Some(Fired { subject, data });
    }
    None
}

/// "Whenever enchanted creature deals damage to you, this Aura deals that much damage to that creature's controller." (CR 608.2, 120.3).
fn aura_deals_much_damage_creatures(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let amount_0 = item.matched_damage_amount();
    let recipients_0 = {
        let q: &dyn Query = g;
        item.subject
            .object()
            .map(|o: ObjectId| q.controller(o))
            .into_iter()
            .map(Target::Player)
            .collect::<Vec<Target>>()
    };
    let batch: Vec<(ObjectId, Target, u32)> = recipients_0
        .iter()
        .map(|t| (item.object, *t, amount_0))
        .collect();
    g.damage_together(&batch);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            when_asked(EventMask::DAMAGE, when_enchanted_creature_deals_damage),
            aura_deals_much_damage_creatures,
            "Whenever enchanted creature deals damage to you, this Aura deals that much damage to that creature's controller.",
        ).damage_amount(),
    ],
    enchant: Some(targets::CREATURE),
    ..oracle::BACKFIRE
};
