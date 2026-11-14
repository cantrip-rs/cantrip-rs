use cantrip_api::*;
use cantrip_mechanics::statics::{attacks_each_combat_if_able, itself};
use cantrip_mechanics::triggers::BEGINNING_OF_YOUR_COMBAT;
use cantrip_mechanics::{effects, ferocious};
use cantrip_oracle as oracle;

static RETURN: Question =
    Question::new("ReturnToBattlefield", "pay {cost} to return {subject} from your graveyard to the battlefield?", "", "Pay {cost}", "Leave it");

/// "You may pay {R}. If you do, return this card from your graveyard to the battlefield."
fn pay_to_return(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::may_pay_to_return_this_to_the_battlefield(g, item, cost("{R}"), &RETURN, &[])
}

/// "Ferocious — At the beginning of combat on your turn, if you control a creature with power 4 or greater" (CR 603.4): an
/// intervening "if", asked as combat begins and again as the ability resolves. It returns this card from the graveyard, so it
/// functions only there (CR 113.6m).
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        BEGINNING_OF_YOUR_COMBAT,
        pay_to_return,
        "Ferocious \u{2014} At the beginning of combat on your turn, if you control a creature with power 4 or greater, you may pay {R}. If you do, return this card from your graveyard to the battlefield.",
    )
    .condition(ferocious::you_control_asked)
    .functions_in(IN_A_GRAVEYARD)],
    statics: &[static_ability(Layer::Ability, itself, attacks_each_combat_if_able, "This creature attacks each combat if able.")],
    ..oracle::FLAMEWAKE_PHOENIX
};
