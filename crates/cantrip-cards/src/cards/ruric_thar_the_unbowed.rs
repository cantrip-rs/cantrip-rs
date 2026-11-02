use cantrip_api::*;
use cantrip_mechanics::statics;
use cantrip_oracle as oracle;

/// "Ruric Thar attacks each combat if able." (CR 604.1).
fn ruric_thar_attacks_combat_if_apply_0(
    _: &dyn LayerQuery,
    _: ObjectId,
    chars: &mut Chars,
) {
    chars.must_attack += 1;
}

/// "Whenever a player casts a noncreature spell" (CR 603.2).
fn when_player_casts_noncreature_spell(
    q: &dyn Query,
    _: &Asker,
    event: &Event,
) -> Option<Fired> {
    let Event::SpellCast { spell, player, .. } = event else { return None };
    let id = *spell;
    let subject = Target::Object(id);
    let data = [u32::from(player.0), u32::MAX];
    if !(q.chars(id).is(CardType::Creature)) {
        return Some(Fired { subject, data });
    }
    None
}

/// "Whenever a player casts a noncreature spell, Ruric Thar deals 6 damage to that player." (CR 608.2, 120.3).
fn ruric_thar_deals_6_damage(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let recipients_0 = Some(PlayerId(item.data[0] as u8))
        .into_iter()
        .map(Target::Player)
        .collect::<Vec<Target>>();
    let batch: Vec<(ObjectId, Target, u16)> = recipients_0
        .iter()
        .map(|t| (item.object, *t, 6u16))
        .collect();
    g.damage_together(&batch);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            when_asked(EventMask::SPELL_CAST, when_player_casts_noncreature_spell),
            ruric_thar_deals_6_damage,
            "Whenever a player casts a noncreature spell, Ruric Thar deals 6 damage to that player.",
        ),
    ],
    statics: &[
        static_ability(
            Layer::Ability,
            statics::itself,
            ruric_thar_attacks_combat_if_apply_0,
            "Ruric Thar attacks each combat if able.",
        ),
    ],
    ..oracle::RURIC_THAR_THE_UNBOWED
};
