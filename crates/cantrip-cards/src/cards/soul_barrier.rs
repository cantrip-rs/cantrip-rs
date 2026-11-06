use cantrip_api::*;
use cantrip_oracle as oracle;

/// "Whenever an opponent casts a creature spell" (CR 603.2).
fn when_opponent_casts_creature_spell(
    q: &dyn Query,
    asker: &Asker,
    event: &Event,
) -> Option<Fired> {
    let Event::SpellCast { spell, player, .. } = event else { return None };
    let id = *spell;
    let subject = Target::Object(id);
    let data = [u32::from(player.0), u32::MAX];
    if q
        .state()
        .opponents(asker.controller)
        .collect::<Vec<PlayerId>>()
        .contains(&*player) && q.chars(id).is(CardType::Creature)
    {
        return Some(Fired { subject, data });
    }
    None
}

/// "Whenever an opponent casts a creature spell, this enchantment deals 2 damage to that player unless they pay {2}." (CR 608.2, 120.3, 118.1).
fn enchantment_deals_2_damage_player(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let paid_0 = g
        .may_pay(
            PlayerId(item.data[0] as u8),
            item.object,
            item.object,
            cost("{2}"),
            &PAY_QUESTION,
        )?;
    g.end_sentence();
    if !(((paid_0 as i64) != 0)) {
        let recipients_1 = Some(PlayerId(item.data[0] as u8))
            .into_iter()
            .map(Target::Player)
            .collect::<Vec<Target>>();
        let batch: Vec<(ObjectId, Target, u32)> = recipients_1
            .iter()
            .map(|t| (item.object, *t, 2))
            .collect();
        g.damage_together(&batch);
        g.end_sentence();
    }
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
            when_asked(EventMask::SPELL_CAST, when_opponent_casts_creature_spell),
            enchantment_deals_2_damage_player,
            "Whenever an opponent casts a creature spell, this enchantment deals 2 damage to that player unless they pay {2}.",
        ),
    ],
    ..oracle::SOUL_BARRIER
};
