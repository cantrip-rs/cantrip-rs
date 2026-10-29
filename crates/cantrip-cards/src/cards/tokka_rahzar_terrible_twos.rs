use cantrip_api::*;
use cantrip_oracle as oracle;

/// "Whenever a player casts a spell, if the amount of mana spent to cast it was less than its mana value" (CR 603.2, 603.4, 601.2h): any
/// player's cast, not a copy (CR 707.10). Its mana value counts X as chosen (CR 202.3e) and the mana spent is all of the payment,
/// additional costs too, so a spell cast for an alternative cost, with a cost reduction or for free has had less spent than its value.
/// The payment is over before the trigger and nothing changes it afterwards, so the condition asked as it triggers is the one asked as it
/// resolves. The spell rides along as the trigger's subject; the player who cast it is "that player".
fn a_player_casts_a_spell_with_less_mana_spent_than_its_mana_value(q: &dyn Query, _: ObjectId, event: &Event) -> Option<Fired> {
    match event {
        Event::SpellCast { spell, .. } if q.obj(*spell).spent.total() < q.mana_value(*spell) => Fired::about(Target::Object(*spell)),
        _ => None,
    }
}

/// "Tokka & Rahzar deal 3 damage to that player."
fn damage_the_caster(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Target::Object(spell) = item.subject else { return Ok(()) };
    if let Some(cast) = g.history().cast_of(spell) {
        g.damage(item.object, Target::Player(cast.player), 3);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        when(EventMask::SPELL_CAST, a_player_casts_a_spell_with_less_mana_spent_than_its_mana_value),
        damage_the_caster,
        "Whenever a player casts a spell, if the amount of mana spent to cast it was less than its mana value, Tokka & Rahzar deal 3 damage to that player.",
    )],
    ..oracle::TOKKA_RAHZAR_TERRIBLE_TWOS
};
