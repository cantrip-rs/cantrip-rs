use cantrip_api::*;
use cantrip_mechanics::statics::{have_indestructible, other_creatures_you_control};
use cantrip_oracle as oracle;

/// The name it becomes (CR 612.8).
const MILEVA: &str = "Mileva, the Stalwart";

const LEGENDARY: Supertypes = Supertypes { basic: false, legendary: true, snow: false, world: false };

/// "This creature becomes a Human Detective with base power and toughness 4/4 and gains vigilance." With no duration, for as long as
/// it stays on the battlefield (CR 611.2a): "becomes a Human Detective" sets its creature types to Human and Detective alone
/// (CR 205.1a, layer 4), base power and toughness 4/4 (CR 613.4b, layer 7b), and vigilance is added in layer 6.
fn become_a_human_detective(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let me = item.object;
    let forever = Duration::Indefinite;
    let human = g.words(me).subtype(Subtype::Human);
    let detective = g.words(me).subtype(Subtype::Detective);
    instruction(g, |g| {
        g.add_effect(me, me, EffectKind::SetCreatureType(human), forever);
        g.add_effect(me, me, EffectKind::AddSubtype(detective), forever);
        g.add_effect(me, me, EffectKind::SetPt(4, 4), forever);
        g.add_effect(me, me, EffectKind::AddKeywords(Keywords::of(&[Keyword::Vigilance])), forever);
    });
    Ok(())
}

/// "If this creature is a Detective, it becomes a legendary creature named Mileva, the Stalwart, it has base power and toughness
/// 5/5, and it gains 'Other creatures you control have indestructible.'" The condition is judged as the ability resolves
/// (CR 608.2c); a Hero that is not a Detective then does nothing. "Becomes a legendary creature" sets its card type to creature
/// alone (CR 205.1a, layer 4; no "in addition to its other types", CR 205.1b), so an earlier effect that made it an artifact no
/// longer does; its creature types stay, and it gains the legendary supertype. The grant is the card's `delayed` static ability 0
/// (CR 113.7, 613.1f): it applies to the other creatures its controller controls while this object is on the battlefield.
fn become_mileva(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let me = item.object;
    let detective = g.words(me).subtype(Subtype::Detective);
    if !g.chars(me).has_subtype(detective) {
        return Ok(());
    }
    let card = g.card_id(&CARD);
    let forever = Duration::Indefinite;
    instruction(g, |g| {
        g.add_effect(me, me, EffectKind::AddSupertypes(LEGENDARY), forever);
        g.add_effect(me, me, EffectKind::SetTypes(TypeSet::of(&[CardType::Creature])), forever);
        g.add_effect(me, me, EffectKind::SetName(MILEVA), forever);
        g.add_effect(me, me, EffectKind::SetPt(5, 5), forever);
        g.add_effect(me, me, EffectKind::AddAbility(AbilityRef::delayed(card, 0)), forever);
    });
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(
            cost("{1}{W}").and(Cost { collect_evidence: 2, ..Cost::NONE }),
            become_a_human_detective,
            "{1}{W}, Collect evidence 2: This creature becomes a Human Detective with base power and toughness 4/4 and gains vigilance.",
        ),
        ability(
            cost("{2}{W}").and(Cost { collect_evidence: 4, ..Cost::NONE }),
            become_mileva,
            "{2}{W}, Collect evidence 4: If this creature is a Detective, it becomes a legendary creature named Mileva, the Stalwart, it has base power and toughness 5/5, and it gains \"Other creatures you control have indestructible.\"",
        ),
    ],
    delayed: &[granted_static_ability(Layer::Ability, other_creatures_you_control, have_indestructible, "Other creatures you control have indestructible.")],
    ..oracle::TENTH_DISTRICT_HERO
};
