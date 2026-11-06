use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;

/// "Whenever a player casts an artifact spell" (CR 603.2).
fn when_player_casts_artifact_spell(
    q: &dyn Query,
    _: &Asker,
    event: &Event,
) -> Option<Fired> {
    let Event::SpellCast { spell, .. } = event else { return None };
    let id = *spell;
    let subject = Target::Object(id);
    if q.chars(id).is(CardType::Artifact) {
        return Fired::about(subject);
    }
    None
}

/// "You may pay {1}. If you do, you gain 1 life."
fn pay_1_if_do_gain(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::may_pay_to_gain_life(g, item, cost("{1}"))
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            when_asked(EventMask::SPELL_CAST, when_player_casts_artifact_spell),
            pay_1_if_do_gain,
            "Whenever a player casts an artifact spell, you may pay {1}. If you do, you gain 1 life.",
        ),
    ],
    ..oracle::URZAS_CHALICE
};
