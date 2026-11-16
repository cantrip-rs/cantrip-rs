use cantrip_api::*;
use cantrip_mechanics::{effects, spree, targets};
use cantrip_oracle as oracle;

/// "Until end of turn, target creature becomes a white Rabbit with base power and toughness 0/1." Three continuous effects of a
/// resolved spell (CR 611.2a), all at its timestamp (CR 613.7a): its creature types replaced by Rabbit, its other types and
/// every ability kept (CR 205.1a, layer 4), white and no other color (CR 105.3, layer 5), and base power and toughness 0/1
/// (layer 7b, so counters and pumps still count on top). They last until the cleanup step (CR 514.2). The color and the creature
/// type are words of the spell's text (CR 612). With the target gone the mode does nothing (CR 608.2b).
fn creature_becomes_a_white_rabbit(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    let words = g.words(item.object);
    let source = item.object;
    instruction(g, |g| {
        g.add_effect(source, creature, EffectKind::SetCreatureType(words.subtype(Subtype::Rabbit)), Duration::EndOfTurn);
        g.add_effect(source, creature, EffectKind::SetColors(words.colors(ColorSet::of(Color::White))), Duration::EndOfTurn);
        g.add_effect(source, creature, EffectKind::SetPt(0, 1), Duration::EndOfTurn);
    });
    Ok(())
}

fn target_player_draws_two(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::target_player_draws(g, item, 0, 2);
    Ok(())
}

const MODES: &[ModeDef] = &[
    mode(
        creature_becomes_a_white_rabbit,
        "+ {1} \u{2014} Until end of turn, target creature becomes a white Rabbit with base power and toughness 0/1.",
    )
    .targets(&[targets::CREATURE])
    .cost(mana("{1}")),
    mode(target_player_draws_two, "+ {3} \u{2014} Target player draws two cards.").targets(&[targets::PLAYER]).cost(mana("{3}")),
];

static MODAL: ModalDef = spree::modal(MODES);

pub static CARD: CardDef = CardDef { spell: Some(SpellDef::MODAL), modal: Some(&MODAL), ..oracle::METAMORPHIC_BLAST };
