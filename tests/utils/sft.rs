use super::{Program, RunResult, FOREIGN_USER};
use ft_logic_io::Action;
use ft_main_io::{FTokenAction, FTokenEvent, InitFToken};
use gstd::{prelude::*, ActorId};
use gtest::{Program as InnerProgram, System};

type SFTRunResult<T> = RunResult<T, FTokenEvent>;

pub struct Sft<'a>(InnerProgram<'a>, u64);

impl Program for Sft<'_> {
    fn inner_program(&self) -> &InnerProgram {
        &self.0
    }
}

impl<'a> Sft<'a> {
    fn transaction_id(&mut self) -> u64 {
        let transaction_id = self.1;
        self.1 = self.1.wrapping_add(1);
        transaction_id
    }

    pub fn initialize(system: &'a System) -> Self {
        let program = InnerProgram::from_file(system, "target/ft_main.opt.wasm");
        let storage_code: [u8; 32] = system.submit_code("target/ft_storage.opt.wasm").into();
        let logic_code: [u8; 32] = system.submit_code("target/ft_logic.opt.wasm").into();

        assert!(!program
            .send(
                FOREIGN_USER,
                InitFToken {
                    storage_code_hash: storage_code.into(),
                    ft_logic_code_hash: logic_code.into(),
                },
            )
            .main_failed());

        Self(program, 0)
    }

    pub fn mint(&mut self, recipient: u64, amount: u128) -> SFTRunResult<FTokenEvent> {
        let transaction_id = self.transaction_id();

        RunResult(
            self.0.send(
                FOREIGN_USER,
                FTokenAction::Message {
                    transaction_id,
                    payload: Action::Mint {
                        recipient: recipient.into(),
                        amount,
                    }
                    .encode(),
                },
            ),
            |event| event,
        )
    }

    pub fn approve(
        &mut self,
        from: u64,
        approved_account: impl Into<ActorId>,
        amount: u128,
    ) -> SFTRunResult<FTokenEvent> {
        let transaction_id = self.transaction_id();

        RunResult(
            self.0.send(
                from,
                FTokenAction::Message {
                    transaction_id,
                    payload: Action::Approve {
                        approved_account: approved_account.into(),
                        amount,
                    }
                    .encode(),
                },
            ),
            |event| event,
        )
    }

    pub fn balance(&self, actor_id: impl Into<ActorId>) -> SFTRunResult<u128> {
        RunResult(
            self.0
                .send(FOREIGN_USER, FTokenAction::GetBalance(actor_id.into())),
            FTokenEvent::Balance,
        )
    }
}
