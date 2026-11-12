use cantrip_api::*;
use cantrip_mechanics::tokens::{self, WIZARD};
use cantrip_mechanics::{effects, permanents_with_subtype_you_control, replacements, triggers};
use cantrip_oracle as oracle;

/// "Create a tapped 0/1 black Wizard creature token with ... Then if you control four or more Wizards, transform Kuja"
/// (CR 111.1, 701.27a): the count is taken once the token is on the battlefield, so the token itself and Kuja count, and
/// the type is read through the layers (a changeling is a Wizard). Transforming does nothing if Kuja has changed faces
/// since the ability was put on the stack (CR 701.27f).
fn make_a_wizard_then_transform(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let you = item.controller;
    let wizard = g.card_id(&WIZARD);
    g.create_tokens(wizard, you, 1, true)?;
    let wizard_type = g.words(item.object).subtype(Subtype::Wizard);
    if permanents_with_subtype_you_control(g, you, wizard_type) >= 4 {
        effects::transform_self(g, item)?;
    }
    Ok(())
}

/// "If a Wizard you control would deal damage to a permanent or player" (CR 614.1a, 120.3): damage, combat or not, from a
/// Wizard its controller controls, to anything, once it has been divided or assigned among its recipients (the rulings: a
/// trampler's 1 and 3 become 2 and 6). Wizard is a word of this card's text (CR 612). Damage already prevented to nothing is
/// no damage (CR 615.6): doubling it leaves nothing. Not just creatures: any Wizard source, a Wizard planeswalker's too.
fn damage_from_a_wizard_you_control(q: &dyn Query, asker: &Asker, p: &Proposal) -> bool {
    let wizard = q.words(asker.source).subtype(Subtype::Wizard);
    matches!(p, Proposal::Damage(d) if q.controller(d.source) == asker.controller && q.chars(d.source).has_subtype(wizard))
}

pub static BACK: CardDef = CardDef {
    replacements: &[replacement(ReplacementKind::Damage, damage_from_a_wizard_you_control, replacements::double_that_damage, "deals double that damage instead")],
    ..oracle::TRANCE_KUJA_FATE_DEFIED
};

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::YOUR_END_STEP,
        make_a_wizard_then_transform,
        "At the beginning of your end step, create a tapped 0/1 black Wizard creature token with \"Whenever you cast a noncreature spell, this token deals 1 damage to each opponent.\" Then if you control four or more Wizards, transform Kuja.",
    )],
    back: Some(&BACK),
    ..oracle::KUJA_GENOME_SORCERER
};

pub static EXTRA: &[&CardDef] = &[&tokens::WIZARD];
