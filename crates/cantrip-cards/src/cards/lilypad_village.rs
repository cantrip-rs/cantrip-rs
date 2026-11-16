use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;

/// "Activate only if a Bird, Frog, Otter, or Rat entered the battlefield under your control this turn" (CR 602.5b): one that
/// entered under this ability's controller's control as it entered (CR 110.2), whatever it has become, wherever it has gone since.
/// Each subtype is read through the words of this card's text (CR 612).
fn a_bird_frog_otter_or_rat_entered_under_your_control(q: &dyn Query, asker: &Asker) -> bool {
    let words = q.words(asker.source);
    let subtypes = [words.subtype(Subtype::Bird), words.subtype(Subtype::Frog), words.subtype(Subtype::Otter), words.subtype(Subtype::Rat)];
    q.history().entered().any(|e| e.controller == asker.controller && subtypes.iter().any(|&s| e.chars(q).has_subtype(s)))
}

/// "Surveil 2."
fn surveil_2(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::surveil(g, item, 2)
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        mana_ability(TAP, add(Mana::C), "{T}: Add {C}."),
        mana_ability(TAP, add(Mana::U).only_for(ManaRestriction::CREATURE_SPELLS), "{T}: Add {U}. Spend this mana only to cast a creature spell."),
        ability(
            cost("{U}, {T}"),
            surveil_2,
            "{U}, {T}: Surveil 2. Activate only if a Bird, Frog, Otter, or Rat entered the battlefield under your control this turn.",
        )
        .condition(a_bird_frog_otter_or_rat_entered_under_your_control),
    ],
    ..oracle::LILYPAD_VILLAGE
};
