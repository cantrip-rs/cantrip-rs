use cantrip_api::*;
use cantrip_oracle as oracle;

/// Redirecting costs life, so neither answer is the default (`Question::yes_is_default`).
static REDIRECT: Question = Question::new("Redirect", "have that damage dealt to you instead?", "", "Redirect", "Decline").no_default();

/// Damage to any creature may be redirected to the spell's controller (CR 614.1a, 614.9).
fn applies(q: &dyn Query, _: &Asker, _: ObjectId, proposal: &Proposal) -> bool {
    matches!(proposal, Proposal::Damage(d) if matches!(d.target, Target::Object(o) if q.chars(o).is(CardType::Creature)))
}

fn redirect(_: &dyn Query, asker: &Asker, _: ObjectId, proposal: &mut Proposal) {
    if let Proposal::Damage(d) = proposal { d.target = Target::Player(asker.controller); }
}

/// The effect changes the rules until cleanup, including for creatures entering later (CR 611.2c).
fn resolve(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.add_player_effect(item.controller, item.object, EffectKind::ReplacementRule { card: item.ability.card, index: 0 }, Duration::EndOfTurn);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[], resolve),
    granted_replacements: &[granted_replacement(ReplacementKind::Damage, applies, redirect, "redirect damage to you")
        .optional(&REDIRECT)
        .text("Until end of turn, if damage would be dealt to any creature, you may have that damage dealt to you instead.")],
    ..oracle::BLOOD_OF_THE_MARTYR
};
