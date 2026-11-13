use cantrip_api::*;
use cantrip_mechanics::{effects, triggers};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(
            cost("{2}{G}{G}"),
            effects::two_plus_one_counters_on_this,
            "Exhaust \u{2014} {2}{G}{G}: Put two +1/+1 counters on this creature. (Activate each exhaust ability only once.)",
        )
        .exhaust(),
        // It returns this card from the graveyard, so it functions only there (CR 113.6m).
        triggered(
            triggers::YOU_ACTIVATE_AN_EXHAUST_ABILITY,
            effects::return_this_card_to_battlefield,
            "Whenever you activate an exhaust ability, return this card from your graveyard to the battlefield.",
        )
        .functions_in(IN_A_GRAVEYARD),
    ],
    ..oracle::AFTERBURNER_EXPERT
};
