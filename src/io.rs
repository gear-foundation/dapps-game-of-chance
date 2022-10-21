use gstd::{prelude::*, ActorId};

#[derive(Debug, Encode, Decode, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, TypeInfo)]
pub struct LotteryInit {
    pub admin: ActorId,
    pub ft_contract: ActorId,
}

#[derive(Debug, Encode, Decode, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, TypeInfo)]
pub enum LotteryAction {
    Start {
        duration: u64,
        participation_cost: u128,
    },
    PickWinner,
    Enter,
}

#[derive(Debug, Encode, Decode, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, TypeInfo)]
pub enum LotteryEvent {
    Started {
        ending: u64,
        participation_cost: u128,
    },
    Winner(ActorId),
    PlayerAdded(ActorId),
}

#[derive(Debug, Encode, Decode, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, TypeInfo)]
pub enum LotteryStateQuery {
    State,
    FTContract,
}

#[derive(Debug, Encode, Decode, PartialEq, Eq, PartialOrd, Ord, Clone, TypeInfo)]
pub enum LotteryStateReply {
    State {
        started: u64,
        ending: u64,
        players: BTreeSet<ActorId>,
        prize_fund: u128,
        participation_cost: u128,
        last_winner: ActorId,
    },
    FTContract(ActorId),
}
