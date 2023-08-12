use std::str::FromStr;

use cosmwasm_std::{coin, Coin, Decimal, Uint128};
use cw_it::cw_multi_test::{ContractWrapper, StargateKeeper, StargateMessageHandler};
use cw_it::multi_test::modules::TokenFactory;
use cw_it::multi_test::MultiTestRunner;
use cw_it::robot::TestRobot;
use cw_it::test_tube::{Account, Module, SigningAccount, Wasm};
use cw_it::traits::CwItRunner;
use cw_it::{Artifact, ContractType, TestRunner};

pub const MOCK_VAULT_TOKEN_SUBDENOM: &str = "vault-token";

const TOKEN_FACTORY: &TokenFactory =
    &TokenFactory::new("factory", 32, 16, 59 + 16, "10000000uosmo");

pub fn get_test_runner<'a>() -> TestRunner<'a> {
    match option_env!("TEST_RUNNER_TYPE").unwrap_or("multi-test") {
        "multi-test" => {
            let mut stargate_keeper = StargateKeeper::new();
            TOKEN_FACTORY.register_msgs(&mut stargate_keeper);

            TestRunner::MultiTest(MultiTestRunner::new_with_stargate("osmo", stargate_keeper))
        }
        #[cfg(feature = "osmosis-test-tube")]
        "osmosis-test-app" => {
            TestRunner::OsmosisTestApp(cw_it::osmosis_test_tube::OsmosisTestApp::new())
        }
        _ => panic!("Unsupported test runner type"),
    }
}

pub const DEFAULT_ARTIFACTS_DIR: &str = "../../artifacts/";

pub fn get_wasm_path(contract_name: &str) -> String {
    let artifacts_dir = option_env!("ARTIFACTS_DIR").unwrap_or(DEFAULT_ARTIFACTS_DIR);

    let mut path = format!("{}/{}", artifacts_dir, contract_name.replace('-', "_"));

    // If path doesn't exist, try appending the CPU architecture
    if !std::path::Path::new(&format!("{}.wasm", path)).exists() {
        path = format!("{}-{}", path, std::env::consts::ARCH);
    }

    format!("{}.wasm", path)
}

pub fn get_mock_vault_contract() -> ContractType {
    match option_env!("TEST_RUNNER_TYPE").unwrap_or("multi-test") {
        "multi-test" => ContractType::MultiTestContract(Box::new(ContractWrapper::new(
            crate::contract::execute,
            crate::contract::instantiate,
            crate::contract::query,
        ))),
        _ => ContractType::Artifact(Artifact::Local(get_wasm_path("mock-vault"))),
    }
}

pub fn assert_almost_eq(left: Decimal, right: Decimal, max_rel_diff: &str) {
    let max_rel_diff = Decimal::from_str(max_rel_diff).unwrap();

    let largest = std::cmp::max(left, right);
    let rel_diff = left.abs_diff(right) / largest;

    if rel_diff > max_rel_diff {
        panic!(
            "assertion failed: `(left ≈ right)`\nleft: {}\nright: {}\nrelative difference: {}\nmax allowed relative difference: {}\n",
            left, right, rel_diff, max_rel_diff
        )
    }
}

/// A trait implementing common methods for testing vault contracts.
pub trait VaultRobot<'a, R>: TestRobot<'a, R>
where
    R: CwItRunner<'a> + 'a,
{
    /// Create a new default instance of the robot.
    fn default_vault_robot(
        runner: &'a R,
        admin: &'a SigningAccount,
        base_token: String,
        vault_token: String,
        vault_addr: String,
    ) -> DefaultVaultRobot<'a, R> {
        DefaultVaultRobot {
            runner,
            admin,
            base_token,
            vault_token,
            vault_addr,
        }
    }

    /// Returns the base token.
    fn base_token(&self) -> &str;

    /// Returns the vault token.
    fn vault_token(&self) -> &str;

    /// Returns the vault address.
    fn vault_addr(&self) -> &str;

    /// Uploads and instantiates the vault contract and returns a new instance of the robot.
    fn instantiate(
        runner: &'a R,
        admin: &'a SigningAccount,
        base_token: &str,
        denom_creation_fee: Option<Coin>,
    ) -> DefaultVaultRobot<'a, R>
    where
        Self: Sized,
    {
        let wasm = Wasm::new(runner);

        let mock_vault = get_mock_vault_contract();
        let code_id = runner.store_code(mock_vault, admin).unwrap();

        let msg = crate::msg::InstantiateMsg {
            base_token: base_token.to_string(),
        };
        let vault_addr = wasm
            .instantiate(
                code_id,
                &msg,
                Some(&admin.address()),
                Some("mock_vault"),
                &denom_creation_fee.map_or_else(|| vec![], |f| vec![f]),
                admin,
            )
            .unwrap()
            .data
            .address;

        let vault_token = format!("factory/{}/{}", vault_addr, MOCK_VAULT_TOKEN_SUBDENOM);

        Self::default_vault_robot(
            runner,
            admin,
            base_token.to_string(),
            vault_token,
            vault_addr,
        )
    }

    /// Deposit base tokens into the vault and return a reference to the robot.
    fn deposit_to_vault(&self, amount: impl Into<Uint128>, signer: &SigningAccount) -> &Self {
        let amount: Uint128 = amount.into();

        let msg = crate::msg::ExecuteMsg::Deposit {
            amount,
            recipient: None,
        };
        self.wasm()
            .execute(
                self.vault_addr(),
                &msg,
                &[coin(amount.u128(), self.base_token())],
                signer,
            )
            .unwrap();

        self
    }

    /// Deposit base tokens into the vault without filling the native token funds field and return
    /// a reference to the robot. This is useful for depositing cw20 tokens.
    fn deposit_cw20_to_vault(&self, amount: impl Into<Uint128>, signer: &SigningAccount) -> &Self {
        let amount: Uint128 = amount.into();

        let msg = crate::msg::ExecuteMsg::Deposit {
            amount,
            recipient: None,
        };
        self.wasm()
            .execute(self.vault_addr(), &msg, &[], signer)
            .unwrap();

        self
    }

    /// Redeem vault tokens from the vault and return a reference to the robot.
    fn redeem_from_vault(&self, amount: impl Into<Uint128>, signer: &SigningAccount) -> &Self {
        let amount: Uint128 = amount.into();

        let msg = crate::msg::ExecuteMsg::Redeem {
            amount,
            recipient: None,
        };
        self.wasm()
            .execute(
                self.vault_addr(),
                &msg,
                &[coin(amount.u128(), self.vault_token())],
                signer,
            )
            .unwrap();

        self
    }

    /// Query the vault token balance of the given account.
    fn query_vault_token_balance(&self, account: impl Into<String>) -> Uint128 {
        self.query_native_token_balance(account, self.vault_token())
    }
}

/// A simple testing robot for testing vault contracts.
pub struct DefaultVaultRobot<'a, R: CwItRunner<'a>> {
    pub runner: &'a R,
    pub admin: &'a SigningAccount,
    pub vault_addr: String,
    pub base_token: String,
    pub vault_token: String,
}

impl<'a, R: CwItRunner<'a>> TestRobot<'a, R> for DefaultVaultRobot<'a, R> {
    fn runner(&self) -> &'a R {
        self.runner
    }
}

impl<'a, R: CwItRunner<'a>> VaultRobot<'a, R> for DefaultVaultRobot<'a, R> {
    fn base_token(&self) -> &str {
        &self.base_token
    }

    fn vault_token(&self) -> &str {
        &self.vault_token
    }

    fn vault_addr(&self) -> &str {
        &self.vault_addr
    }
}
