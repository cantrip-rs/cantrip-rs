//! Havengul Lich can let a player cast another player's creature from that player's graveyard.
//! Its permission is represented by a test-only source alongside the actual Amalgam.
//! Nobody both cast that creature and owned its graveyard, so Amalgam must not trigger (CR 603.4).

use std::sync::OnceLock;

use cantrip_api::Game as Rules;
use cantrip_engine::*;
use cantrip_script::{GameScript, run_on};

/// The already-resolved permission from Havengul Lich, without implementing the unrelated card.
fn permit(g: &mut dyn Rules, item: &StackItem) -> Result<(), Suspend> {
    if let Some(card) = cantrip_mechanics::targets::object(g, item, 0) {
        assert_eq!(item.controller, PlayerId(1));
        assert_eq!(g.state().obj(card).zone, Zone::graveyard(PlayerId(0)));
        g.allow_play(card, item.controller, PlayGrant { expiry: Expiry::EndOfTurn, ..PlayGrant::CAST });
        assert!(g.state().permissions.iter().any(|p| p.player == item.controller && g.state().permission_covers(p, card)));
    }
    Ok(())
}

static PERMISSION: CardDef = CardDef {
    name: "Probe Havengul Permission",
    oracle: true,
    has_cost: true,
    types: TypeSet::of(&[CardType::Artifact]),
    abilities: &[
        ability(cost("{0}"), permit, "You may cast target creature card from a graveyard this turn.")
            .targets(&[cantrip_mechanics::targets::CREATURE_CARD_IN_A_GRAVEYARD]),
    ],
    ..CardDef::BLANK
};

fn table() -> &'static CardTable {
    static TABLE: OnceLock<CardTable> = OnceLock::new();
    TABLE.get_or_init(|| CardTable::new(cantrip_cards::all().into_iter().chain([&PERMISSION])))
}

#[test]
fn the_trigger_does_not_confuse_the_caster_with_the_graveyard_owner() {
    let script = GameScript::parse(
        "
[Bot1 Deck]
Mox Ruby
Faithless Looting
Prized Amalgam
Grizzly Bears
Island x20
[Bot2 Deck]
Black Lotus
Mox Ruby
Probe Havengul Permission
Faithless Looting
Prized Amalgam
Island x20
[Actions]
Bot1.Hand.Mox Ruby.Cast
Bot1.Battlefield.Mox Ruby.Activate
Bot1.Hand.Faithless Looting.Cast
Bot1.Pass
Bot2.Pass
Bot1.Choose(Prized Amalgam; Grizzly Bears)
[T2 Bot2.Main1]
Bot2.Hand.Black Lotus.Cast
Bot2.Hand.Mox Ruby.Cast
Bot2.Battlefield.Mox Ruby.Activate
Bot2.Hand.Faithless Looting.Cast
Bot2.Pass
Bot1.Pass
Bot2.Choose(Prized Amalgam; Island)
Bot2.Hand.Probe Havengul Permission.Cast
Bot2.Battlefield.Probe Havengul Permission.Activate(Bot1.Graveyard.Grizzly Bears)
Bot2.Pass
Bot1.Pass
Bot2.Battlefield.Black Lotus#1.Activate(Color: Green)
Bot2.Grizzly Bears.Cast
Bot2.Pass
Bot1.Pass
? Stack.Count = 0
[T2 Bot2.End]
Bot2.Pass
Bot1.Pass
[Expect]
Bot1.Graveyard.Prized Amalgam.Power = 3
Bot2.Graveyard.Prized Amalgam.Power = 3
Bot1.Battlefield.Prized Amalgam = (absent)
Bot2.Battlefield.Prized Amalgam = (absent)
Bot2.Battlefield.Grizzly Bears.Power = 2
",
        "prized-amalgam-borrowed-graveyard",
    )
    .expect("valid script");
    if let Err(failure) = run_on(&script, table()) {
        panic!("{}", failure.report);
    }
}
