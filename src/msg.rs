#[cfg(feature = "force-unlock")]
use crate::extensions::force_unlock::ForceUnlockExecuteMsg;
#[cfg(feature = "keeper")]
use crate::extensions::keeper::{KeeperExecuteMsg, KeeperQueryMsg};
#[cfg(feature = "lockup")]
use crate::extensions::lockup::{LockupExecuteMsg, LockupQueryMsg};

use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Empty, Uint128};
use schemars::JsonSchema;

/// The default ExecuteMsg variants that all vaults must implement.
/// This enum can be extended with additional variants by defining an extension
/// enum and then passing it as the generic argument `T` to this enum.
#[cw_serde]
pub enum VaultStandardExecuteMsg<T = ExtensionExecuteMsg> {
    /// Called to deposit into the vault. Native assets are passed in the funds
    /// parameter.
    Deposit {
        /// The amount of base tokens to deposit.
        amount: Uint128,
        /// The optional recipient of the vault token. If not set, the caller
        /// address will be used instead.
        recipient: Option<String>,
    },

    /// Called to redeem vault tokens and receive assets back from the vault.
    /// The native vault token must be passed in the funds parameter, unless the
    /// lockup extension is called, in which case the vault token has already
    /// been passed to ExecuteMsg::Unlock.
    Redeem {
        /// An optional field containing which address should receive the
        /// withdrawn base tokens. If not set, the caller address will be
        /// used instead.
        recipient: Option<String>,
        /// The amount of vault tokens sent to the contract. In the case that
        /// the vault token is a Cosmos native denom, we of course have this
        /// information in the info.funds, but if the vault implements the
        /// Cw4626 API, then we need this argument. We figured it's
        /// better to have one API for both types of vaults, so we
        /// require this argument.
        amount: Uint128,
    },

    /// Called to execute functionality of any enabled extensions.
    VaultExtension(T),
}

/// Contains ExecuteMsgs of all enabled extensions. To enable extensions defined
/// outside of this create, you can define your own `ExtensionExecuteMsg` type
/// in your contract crate and pass it in as the generic parameter to ExecuteMsg
#[cw_serde]
pub enum ExtensionExecuteMsg {
    #[cfg(feature = "keeper")]
    Keeper(KeeperExecuteMsg),
    #[cfg(feature = "lockup")]
    Lockup(LockupExecuteMsg),
    #[cfg(feature = "force-unlock")]
    ForceUnlock(ForceUnlockExecuteMsg),
}

/// The default QueryMsg variants that all vaults must implement.
/// This enum can be extended with additional variants by defining an extension
/// enum and then passing it as the generic argument `T` to this enum.
#[cw_serde]
#[derive(QueryResponses)]
pub enum VaultStandardQueryMsg<T = ExtensionQueryMsg>
where
    T: JsonSchema,
{
    /// Returns `VaultStandardInfo` with information on the version of the vault
    /// standard used as well as any enabled extensions.
    #[returns(VaultStandardInfoResponse)]
    VaultStandardInfo {},

    /// Returns `VaultInfo` representing vault requirements, lockup, & vault
    /// token denom.
    #[returns(VaultInfoResponse)]
    Info {},

    /// Returns `Uint128` amount of vault tokens that will be returned for the
    /// passed in assets.
    ///
    /// Allows an on-chain or off-chain user to simulate the effects of their
    /// deposit at the current block, given current on-chain conditions.
    ///
    /// MUST return as close to and no more than the exact amount of Vault
    /// shares that would be minted in a deposit call in the same transaction.
    /// I.e. deposit should return the same or more shares as previewDeposit if
    /// called in the same transaction.
    ///
    /// MUST NOT account for deposit limits like those returned from maxDeposit
    /// and should always act as though the deposit would be accepted,
    /// regardless if the user has enough tokens approved, etc.
    ///
    /// MUST be inclusive of deposit fees. Integrators should be aware of the
    /// existence of deposit fees.
    #[returns(Uint128)]
    PreviewDeposit { amount: Uint128 },

    /// Returns the number of base tokens that would be redeemed in exchange
    /// `amount` for vault tokens. Used by Rover to calculate vault position
    /// values.
    #[returns(Uint128)]
    PreviewRedeem { amount: Uint128 },

    /// Returns the amount of assets managed by the vault denominated in base
    /// tokens. Useful for display purposes, and does not have to confer the
    /// exact amount of base tokens.
    #[returns(Uint128)]
    TotalAssets {},

    /// Returns `Uint128` total amount of vault tokens in circulation.
    #[returns(Uint128)]
    TotalVaultTokenSupply {},

    /// The amount of shares that the vault would exchange for the amount of
    /// assets provided, in an ideal scenario where all the conditions are met.
    ///
    /// Useful for display purposes and does not have to confer the exact amount
    /// of shares returned by the vault if the passed in assets were deposited.
    /// This calculation may not reflect the “per-user” price-per-share, and
    /// instead should reflect the “average-user’s” price-per-share, meaning
    /// what the average user should expect to see when exchanging to and from.
    #[returns(Uint128)]
    ConvertToShares { amount: Uint128 },

    /// Returns the amount of base tokens that the Vault would exchange for
    /// the `amount` of shares provided, in an ideal scenario where all the
    /// conditions are met.
    ///
    /// Useful for display purposes and does not have to confer the exact amount
    /// of assets returned by the vault if the passed in shares were withdrawn.
    /// This calculation may not reflect the “per-user” price-per-share, and
    /// instead should reflect the “average-user’s” price-per-share, meaning
    /// what the average user should expect to see when exchanging to and from.
    #[returns(Uint128)]
    ConvertToAssets { amount: Uint128 },

    /// Handle quries of any enabled extensions.
    #[returns(Empty)]
    VaultExtension(T),
}

/// Contains QueryMsgs of all enabled extensions. To enable extensions defined
/// outside of this create, you can define your own `ExtensionQueryMsg` type
/// in your contract crate and pass it in as the generic parameter to QueryMsg
#[cw_serde]
pub enum ExtensionQueryMsg {
    #[cfg(feature = "keeper")]
    Keeper(KeeperQueryMsg),
    #[cfg(feature = "lockup")]
    Lockup(LockupQueryMsg),
}

/// Struct returned from QueryMsg::VaultStandardInfo with information about the
/// used version of the vault standard and any extensions used.
///
/// This struct should be stored as an Item under the `vault_standard_info` key,
/// so that other contracts can do a RawQuery and read it directly from storage
/// instead of needing to do a costly SmartQuery.
#[cw_serde]
pub struct VaultStandardInfoResponse {
    /// The version of the vault standard used. A number, e.g. 1, 2, etc.
    pub version: u16,
    /// A list of vault standard extensions used by the vault.
    /// E.g. ["lockup", "keeper"]
    pub extensions: Vec<String>,
}

/// Returned by QueryMsg::Info and contains information about this vault
#[cw_serde]
pub struct VaultInfoResponse {
    /// The token that is accepted for deposits, withdrawals and used for
    /// accounting in the vault. The denom if it is a native token and the
    /// contract address if it is a cw20 token.
    pub base_token: String,
    /// Vault token. The denom if it is a native token and the contract address
    /// if it is a cw20 token.
    pub vault_token: String,
}
