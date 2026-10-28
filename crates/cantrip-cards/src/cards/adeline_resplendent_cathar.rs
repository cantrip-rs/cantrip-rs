use cantrip_api::*;
use cantrip_mechanics::combat::{choose_among, create_attacking, player_or_their_planeswalkers};
use cantrip_mechanics::{statics, tokens, triggers};
use cantrip_oracle as oracle;

/// "For each opponent, create a 1/1 white Human creature token that's tapped and attacking that player or a planeswalker
/// they control" (CR 508.4): its controller chooses which, asked only when the opponent controls a planeswalker.
fn human_for_each_opponent(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let human = g.card_id(&tokens::HUMAN);
    let opponents: Vec<PlayerId> = g.state().opponents(item.controller).collect();
    let mut chosen: Vec<Target> = Vec::new();
    for opponent in opponents {
        let candidates = player_or_their_planeswalkers(g, opponent);
        chosen.extend(choose_among(g, item.controller, item.object, &candidates)?);
    }
    create_attacking(g, item.controller, human, true, &chosen)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    statics: &[characteristic_defining(Layer::PtDefine, statics::power_is_creatures_you_control, "Adeline's power is equal to the number of creatures you control.").lost_pt(0, 4)],
    abilities: &[triggered(triggers::YOU_ATTACK, human_for_each_opponent, "Whenever you attack, for each opponent, create a 1/1 white Human creature token that's tapped and attacking that player or a planeswalker they control.")],
    ..oracle::ADELINE_RESPLENDENT_CATHAR
};

pub static EXTRA: &[&CardDef] = &[&tokens::HUMAN];
