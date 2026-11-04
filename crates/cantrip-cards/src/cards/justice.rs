use cantrip_api::*;
use cantrip_mechanics::triggers;
use cantrip_oracle as oracle;

/// Sacrifice unless the controller pays {W}{W} (CR 118.12a, 701.21a).
fn upkeep(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if !g.may_pay(item.controller, item.object, item.object, cost("{W}{W}"), &PAY_QUESTION)? {
        g.sacrifice(item.controller, &[item.object]);
    }
    Ok(())
}

/// A red creature deals damage (CR 603.2).
fn when_beginning_upkeep_2(
    q: &dyn Query,
    asker: &Asker,
    event: &Event,
) -> Option<Fired> {
    let Event::Damage { source, .. } = event else { return None };
    let source = *source;
    let subject = Target::Object(source);
    let data = [0, source.0];
    if !q.state().obj(source).zone.is(ZoneKind::Stack)
        && (q.chars(source).colors.has(q.words(asker.source).color(Color::Red))
            && q.chars(source).is(CardType::Creature))
    {
        return Some(Fired { subject, data });
    }
    None
}

/// Justice deals the damage to the source creature’s controller (CR 608.2, 120.3).
fn sacrifice_enchantment_unless_pay_w_2(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let amount_4 = item.matched_damage_amount();
    let recipients_4 = {
        let q: &dyn Query = g;
        item.subject
            .object()
            .map(|o: ObjectId| q.controller(o))
            .into_iter()
            .map(Target::Player)
            .collect::<Vec<Target>>()
    };
    let batch: Vec<(ObjectId, Target, u32)> = recipients_4
        .iter()
        .map(|t| (item.object, *t, amount_4 as u32))
        .collect();
    g.damage_together(&batch);
    Ok(())
}

/// "Whenever a red creature or spell deals damage" (CR 603.2).
fn when_red_creature_spell_deals_damage(
    q: &dyn Query,
    asker: &Asker,
    event: &Event,
) -> Option<Fired> {
    let Event::Damage { source, .. } = event else { return None };
    let source = *source;
    let subject = Target::Object(source);
    let data = [0, source.0];
    if q.state().obj(source).zone.is(ZoneKind::Stack)
        && q.chars(source).colors.has(q.words(asker.source).color(Color::Red))
    {
        return Some(Fired { subject, data });
    }
    None
}

/// "Whenever a red creature or spell deals damage, this enchantment deals that much damage to that creature's or spell's controller." (CR 608.2, 120.3).
fn enchantment_deals_much_damage_creatures(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let amount_5 = item.matched_damage_amount();
    let recipients_5 = {
        let q: &dyn Query = g;
        item.subject
            .object()
            .map(|o: ObjectId| q.controller(o))
            .into_iter()
            .map(Target::Player)
            .collect::<Vec<Target>>()
    };
    let batch: Vec<(ObjectId, Target, u32)> = recipients_5
        .iter()
        .map(|t| (item.object, *t, amount_5 as u32))
        .collect();
    g.damage_together(&batch);
    Ok(())
}

static PAY_QUESTION: Question = Question::new(
    "Pay",
    "pay {cost}?",
    "",
    "Pay {cost}",
    "Decline",
);

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            triggers::YOUR_UPKEEP,
            upkeep,
            "At the beginning of your upkeep, sacrifice this enchantment unless you pay {W}{W}.",
        ),
        triggered(
                when_asked(EventMask::DAMAGE, when_beginning_upkeep_2),
                sacrifice_enchantment_unless_pay_w_2,
                "Whenever a red creature or spell deals damage, this enchantment deals that much damage to that creature's or spell's controller.",
            )
            .damage_amount()
            .sums_per_subject(),
        triggered(
                when_asked(EventMask::DAMAGE, when_red_creature_spell_deals_damage),
                enchantment_deals_much_damage_creatures,
                "Whenever a red creature or spell deals damage, this enchantment deals that much damage to that creature's or spell's controller.",
            )
            .damage_amount()
            .sums_per_subject(),
    ],
    ..oracle::JUSTICE
};
