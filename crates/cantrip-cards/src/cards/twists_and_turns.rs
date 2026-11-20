use cantrip_api::*;
use cantrip_mechanics::{effects, look_for_card_type, targets, triggers, you_control_at_least_n_lands};
use cantrip_oracle as oracle;

/// "If a creature you control would explore" (CR 614.1a, 701.44): the creature, even one that has left the battlefield by the time
/// it explores (CR 701.44c), under the controller it had. A permanent that is not a creature as it explores still explores (the
/// rulings), but it is not a creature you control, so its explore is not replaced.
fn your_creature_would_explore(q: &dyn Query, asker: &Asker, proposal: &Proposal) -> bool {
    matches!(proposal, Proposal::Explore(e) if q.state().obj(e.creature).controller == asker.controller && q.chars(e.creature).is(CardType::Creature))
}

/// "... instead you scry 1, then that creature explores": the explore goes on as it was, after the scry, which is what the effect does in
/// place of it (CR 614.6). Each such effect applies to it once (CR 614.5): two of them scry twice (the ruling).
fn the_explore_goes_on(_: &dyn Query, _: &Asker, _: &mut Proposal) {}

fn you_scry_one(g: &mut dyn Game, asker: &Asker, _: &Proposal, _: &Proposal) {
    let _ = cantrip_mechanics::scry(g, asker.controller, asker.source, 1);
}

/// "Look at the top four cards of your library. You may reveal a creature card from among them and put that card into your hand. Put the
/// rest on the bottom of your library in a random order." (CR 701.20a, 401.4): with no creature card among them the four are looked at
/// and all go to the bottom.
fn look_for_a_creature(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    look_for_card_type(g, item.controller, item.object, 4, CardType::Creature)
}

pub static BACK: CardDef = CardDef {
    abilities: &[
        mana_ability(TAP, add(Mana::G), "{T}: Add {G}."),
        ability(
            cost("{3}{G}").and(TAP),
            look_for_a_creature,
            "{3}{G}, {T}: Look at the top four cards of your library. You may reveal a creature card from among them and put that card into your hand. Put the rest on the bottom of your library in a random order.",
        ),
    ],
    ..oracle::MYCOID_MAZE
};

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(triggers::THIS_ENTERS, effects::target_creature_explores, "When this enchantment enters, target creature you control explores.")
            .targets(&[targets::CREATURE_YOU_CONTROL]),
        triggered(
            triggers::LANDFALL,
            effects::transform_self,
            "When a land you control enters, if you control seven or more lands, transform this enchantment.",
        )
        .condition(you_control_at_least_n_lands::<7>),
    ],
    replacements: &[replacement(
        ReplacementKind::Explore,
        your_creature_would_explore,
        the_explore_goes_on,
        "you scry 1, then it explores",
    )
    .then(you_scry_one)],
    back: Some(&BACK),
    ..oracle::TWISTS_AND_TURNS
};
