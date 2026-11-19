use cantrip_api::*;
use cantrip_mechanics::{cost_reduction, effects, targets, triggers};
use cantrip_oracle as oracle;

/// The Adventure (CR 715): "Put target instant or sorcery card from your graveyard on top of your library" (CR 401.4), only if the
/// card is still a legal target as the spell resolves (CR 608.2b); the Adventure's own exile is the spell's last step (CR 715.3d).
pub static BUBBLE_UP: CardDef = CardDef {
    spell: spell(&[targets::INSTANT_OR_SORCERY_IN_YOUR_GRAVEYARD], effects::put_first_target_on_top_of_library),
    ..oracle::BUBBLE_UP
};

fn instant_or_sorcery(chars: &Chars) -> bool {
    chars.is(CardType::Instant) || chars.is(CardType::Sorcery)
}

/// "Whenever you cast your first instant or sorcery spell each turn" (CR 603.2): the spell is an instant or sorcery its controller
/// casts and no other instant or sorcery spell was cast by them this turn, one cast before the Alchemist entered included; creature
/// spells and opponents' spells do not count.
fn first_instant_or_sorcery_you_cast(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    let fired = triggers::you_cast_instant_or_sorcery(q, me, event)?;
    let Event::SpellCast { spell, player, .. } = event else { return None };
    cost_reduction::first_spell_you_cast_this_turn(q, *spell, *player, instant_or_sorcery).then_some(fired)
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        when_asked(EventMask::SPELL_CAST, first_instant_or_sorcery_you_cast),
        effects::this_gets::<2, 0>,
        "Whenever you cast your first instant or sorcery spell each turn, this creature gets +2/+0 until end of turn.",
    )],
    adventure: Some(&BUBBLE_UP),
    ..oracle::AQUATIC_ALCHEMIST
};

pub static EXTRA: &[&CardDef] = &[&BUBBLE_UP];
