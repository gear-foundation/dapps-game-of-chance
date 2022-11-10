use common::{InitResult, MetaStateReply, Program, RunResult};
use gstd::{prelude::*, ActorId};
use gtest::{Program as InnerProgram, System};
use rand::{RngCore, SeedableRng};
use rand_xoshiro::Xoshiro128PlusPlus;

use lottery::*;

mod sft;

pub mod common;
pub mod prelude;

pub use common::initialize_system;
pub use sft::Sft;

pub const FOREIGN_USER: u64 = 9999999;

type LotteryRunResult<T> = RunResult<T, LotteryEvent>;
type LotteryInitResult<'a> = InitResult<Lottery<'a>>;

pub struct Lottery<'a>(InnerProgram<'a>);

impl Program for Lottery<'_> {
    fn inner_program(&self) -> &InnerProgram {
        &self.0
    }
}

impl<'a> From<InnerProgram<'a>> for Lottery<'a> {
    fn from(program: InnerProgram<'a>) -> Self {
        Self(program)
    }
}

impl<'a> Lottery<'a> {
    pub fn initialize(system: &'a System, admin: impl Into<ActorId>) -> LotteryInitResult {
        let program = InnerProgram::current(system);

        let failed = program
            .send(
                FOREIGN_USER,
                LotteryInit {
                    admin: admin.into(),
                },
            )
            .main_failed();

        InitResult(Self(program), failed)
    }

    pub fn meta_state(&self) -> LotteryMetaState {
        LotteryMetaState(&self.0)
    }

    pub fn start(
        &mut self,
        from: u64,
        duration: u64,
        participation_cost: u128,
        ft_address: Option<ActorId>,
    ) -> LotteryRunResult<(u64, u128, Option<ActorId>)> {
        RunResult(
            self.0.send(
                from,
                LotteryAction::Start {
                    duration,
                    participation_cost,
                    ft_actor_id: ft_address,
                },
            ),
            |(ending, participation_cost, ft_address)| LotteryEvent::Started {
                ending,
                participation_cost,
                ft_actor_id: ft_address,
            },
        )
    }

    pub fn enter(&mut self, from: u64) -> LotteryRunResult<u64> {
        self.enter_with_value(from, 0)
    }

    pub fn enter_with_value(&mut self, from: u64, value: u128) -> LotteryRunResult<u64> {
        RunResult(
            self.0.send_with_value(from, LotteryAction::Enter, value),
            |actor_id| LotteryEvent::PlayerAdded(actor_id.into()),
        )
    }

    pub fn pick_winner(&mut self, from: u64) -> LotteryRunResult<ActorId> {
        RunResult(
            self.0.send(from, LotteryAction::PickWinner),
            LotteryEvent::Winner,
        )
    }
}

pub struct LotteryMetaState<'a>(&'a InnerProgram<'a>);

impl LotteryMetaState<'_> {
    pub fn state(self) -> MetaStateReply<LotteryStateReply> {
        MetaStateReply(
            self.0
                .meta_state(LotteryStateQuery::State)
                .expect("Failed to decode `LotteryStateReply`"),
        )
    }
}

pub fn predict_winner(system: &System, players: &[u64]) -> u64 {
    let mut random_data = [0; 4];

    Xoshiro128PlusPlus::seed_from_u64(system.block_timestamp()).fill_bytes(&mut random_data);

    let mystical_number = u32::from_le_bytes(random_data) as usize;

    players[mystical_number % players.len()]
}
