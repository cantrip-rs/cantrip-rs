use cantrip_api::*;
use cantrip_mechanics::COUNTERED_UNLESS_PAID;
use cantrip_oracle as oracle;

/// The spell and the player who cast it are fixed at the cast event (CR 603.2).
fn when_player_casts_spell(_: &dyn Query, _: &Asker, event: &Event) -> Option<Fired> {
    let Event::SpellCast { spell, player, .. } = event else { return None };
    Some(Fired { subject: Target::Object(*spell), data: [u32::from(player.0), u32::MAX] })
}

/// The caster may pay as this ability resolves; otherwise counter the spell (CR 118.12a, 701.6a).
fn counter_unless_player_pays_3(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(spell) = item.subject.object() else { return Ok(()) };
    let payer = PlayerId(item.data[0] as u8);
    if !g.may_pay(payer, item.object, spell, cost("{3}"), &COUNTERED_UNLESS_PAID)? {
        g.counter(spell);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        when_asked(EventMask::SPELL_CAST, when_player_casts_spell),
        counter_unless_player_pays_3,
        "Whenever a player casts a spell, counter it unless that player pays {3}.",
    )],
    ..oracle::NETHER_VOID
};
