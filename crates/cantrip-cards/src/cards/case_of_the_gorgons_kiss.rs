use cantrip_api::*;
use cantrip_mechanics::lands::sub;
use cantrip_mechanics::{effects, history, solved, statics, targets, triggers};
use cantrip_oracle as oracle;

/// "To solve \u{2014} Three or more creature cards were put into graveyards from anywhere this turn" (CR 719.3a): every
/// creature card, counted once each, that any move into any player's graveyard made this turn, a destroyed, sacrificed, discarded or
/// milled one alike and a token never (CR 111.7). An intervening "if" (CR 603.4), asked as the end step begins and as the ability
/// resolves.
fn three_creature_cards_were_put_into_graveyards(q: &dyn Query, _: &Asker) -> bool {
    history::creature_cards_put_into_graveyards_this_turn(q, None) >= 3
}

/// The affected object of "Solved \u{2014} This Case is a 4/4 Gorgon creature ... in addition to its other types" (CR 702.169b):
/// the Case itself, while it is solved.
fn this_case_while_solved(view: &dyn LayerQuery, source: ObjectId, affected: ObjectId, _: &Chars) -> bool {
    source == affected && solved::solved_in_layers(view, source)
}

/// "This Case is a 4/4 Gorgon creature ... in addition to its other types" (CR 205.1b, 613.1d): the type and the subtype, one layer-4 effect
/// (the later ones continue it, CR 613.6). "Gorgon" is a word of the Case's text (CR 612).
fn is_a_gorgon_creature_too(view: &dyn LayerQuery, source: ObjectId, chars: &mut Chars) {
    statics::creature_too(view, source, chars);
    statics::also_a::<{ sub(Subtype::Gorgon) }>(view, source, chars);
}

const SOLVED: &str = "Solved \u{2014} This Case is a 4/4 Gorgon creature with deathtouch and lifelink in addition to its other types.";

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(triggers::THIS_ENTERS, effects::destroy_first_target, "When this Case enters, destroy up to one target creature that was dealt damage this turn.")
            .targets(&[targets::CREATURE_THAT_WAS_DEALT_DAMAGE_THIS_TURN.between(0, 1)]),
        solved::to_solve(
            three_creature_cards_were_put_into_graveyards,
            "To solve \u{2014} Three or more creature cards were put into graveyards from anywhere this turn. (If unsolved, solve at the beginning of your end step.)",
        ),
    ],
    statics: &[
        static_ability(Layer::Type, this_case_while_solved, is_a_gorgon_creature_too, SOLVED),
        static_ability(Layer::Ability, this_case_while_solved, statics::have_deathtouch_and_lifelink, SOLVED).continues(),
        static_ability(Layer::PtSet, this_case_while_solved, statics::base_pt::<4, 4>, SOLVED).continues(),
    ],
    ..oracle::CASE_OF_THE_GORGONS_KISS
};
