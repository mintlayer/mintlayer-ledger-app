/*****************************************************************************
 *   Mintlayer Ledger App.
 *   (c) 2025-2026 RBB S.r.l.
 *
 *  Licensed under the Apache License, Version 2.0 (the "License");
 *  you may not use this file except in compliance with the License.
 *  You may obtain a copy of the License at
 *
 *      http://www.apache.org/licenses/LICENSE-2.0
 *
 *  Unless required by applicable law or agreed to in writing, software
 *  distributed under the License is distributed on an "AS IS" BASIS,
 *  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 *  See the License for the specific language governing permissions and
 *  limitations under the License.
 *****************************************************************************/

use alloc::collections::BTreeMap;

use mintlayer_messages::{
    AccountCommand, AccountSpending, AdditionalOrderInfo, AdditionalUtxoInfo, Amount,
    OrderAccountCommand, OutputValue, TokenId, TxInputWithAdditionalInfo, TxOutput,
};

use crate::StatusWord;

#[derive(Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum CoinOrTokenId {
    Coin,
    TokenId(TokenId),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TxType {
    Transfer,
    Burn,
    Htlc,
    CreateDelegation,
    DelegateStaking,
    DelegationWithdrawal,
    CreateStakePool,
    DecommissionStakePool,
    CreateNft,
    CreateToken,
    MintTokens,
    UnmintTokens,
    FreezeToken,
    UnfreezeToken,
    LockTokenSupply,
    ChangeTokenAuthority,
    ChangeTokenMetadataUri,
    FillOrder,
    FreezeOrder,
    CreateOrder,
    ConcludeOrder,
    DataDeposit,
    ComplexTransaction,
}

pub enum InputCommand {
    AccountSpending(AccountSpending),
    AccountCommand(AccountCommand),
    OrderCommand(OrderAccountCommand, AdditionalOrderInfo),
}

pub struct TxSummaryCollector {
    tx_type: Option<TxType>,
    input_command: Option<InputCommand>,
    total_inputs: BTreeMap<CoinOrTokenId, Amount>,
    total_outputs: BTreeMap<CoinOrTokenId, Amount>,
}

impl TxSummaryCollector {
    pub fn new() -> Self {
        Self {
            tx_type: None,
            input_command: None,
            total_inputs: BTreeMap::new(),
            total_outputs: BTreeMap::new(),
        }
    }

    // TODO:
    // 1) currently consensus only forbids multiple account commands (AccountCommand or OrderAccountCommand) per tx,
    //    but the number of account spendings is unlimited and they can co-exist with an account command;
    // 2) probably the app shouldn't try being smart and assume any number of account commands is possible, asking
    //    the user to approve them as they arrive, same as it's done for outputs;
    // 3) if the app does try to be smart, then it should fail when multiple commands are encountered, instead of
    //    silently overwriting `input_command`.
    // See https://github.com/mintlayer/mintlayer-ledger-app/issues/14.
    // Also see the TODO near `TxProcessingContext::show_spinner`.
    pub fn input_command(&self) -> Option<&InputCommand> {
        self.input_command.as_ref()
    }

    pub fn tx_type(&self) -> Option<TxType> {
        self.tx_type
    }

    pub fn total_inputs(&self) -> &BTreeMap<CoinOrTokenId, Amount> {
        &self.total_inputs
    }

    pub fn total_outputs(&self) -> &BTreeMap<CoinOrTokenId, Amount> {
        &self.total_outputs
    }

    pub fn fees_iter(
        &self,
    ) -> impl Iterator<Item = Result<(&CoinOrTokenId, u128), StatusWord>> + '_ {
        // TODO: if an asset is only present in total_outputs, this will not fail with TxFeeUnderflow,
        // but it should.
        // See https://github.com/mintlayer/mintlayer-ledger-app/issues/15.
        self.total_inputs()
            .iter()
            .map(move |(coin_or_token, amount)| {
                let out = *self
                    .total_outputs()
                    .get(coin_or_token)
                    .unwrap_or(&Amount::ZERO);

                let fee = amount
                    .into_atoms()
                    .checked_sub(out.into_atoms())
                    .ok_or(StatusWord::TxFeeUnderflow)?;

                Ok((coin_or_token, fee))
            })
    }

    pub fn process_output(&mut self, out: &TxOutput) -> Result<(), StatusWord> {
        match &out {
            TxOutput::Transfer(value, _) | TxOutput::LockThenTransfer(value, _, _) => {
                self.tx_type = merge_tx_type(self.tx_type, TxType::Transfer);

                let (coin_or_token_id, amount) = into_coin_or_token_id_and_amount(value)?;
                self.increase_output_totals(coin_or_token_id, amount)?;
            }
            TxOutput::Burn(value) => {
                self.tx_type = merge_tx_type(self.tx_type, TxType::Burn);

                let (coin_or_token_id, amount) = into_coin_or_token_id_and_amount(value)?;
                self.increase_output_totals(coin_or_token_id, amount)?;
            }
            TxOutput::Htlc(value, _) => {
                self.tx_type = merge_tx_type(self.tx_type, TxType::Htlc);

                let (coin_or_token_id, amount) = into_coin_or_token_id_and_amount(value)?;
                self.increase_output_totals(coin_or_token_id, amount)?;
            }
            TxOutput::CreateStakePool(_, data) => {
                self.tx_type = merge_tx_type(self.tx_type, TxType::CreateStakePool);

                self.increase_output_totals(CoinOrTokenId::Coin, data.pledge)?;
            }
            TxOutput::ProduceBlockFromStake(_, _) => {}
            TxOutput::DelegateStaking(amount, _) => {
                self.tx_type = merge_tx_type(self.tx_type, TxType::DelegateStaking);
                self.increase_output_totals(CoinOrTokenId::Coin, *amount)?;
            }
            TxOutput::CreateDelegationId(_, _) => {
                self.tx_type = merge_tx_type(self.tx_type, TxType::CreateDelegation);
            }
            TxOutput::IssueFungibleToken(_) => {
                self.tx_type = merge_tx_type(self.tx_type, TxType::CreateToken);
            }
            TxOutput::DataDeposit(_) => {
                self.tx_type = merge_tx_type(self.tx_type, TxType::DataDeposit);
            }
            TxOutput::IssueNft(_, _, _) => {
                self.tx_type = merge_tx_type(self.tx_type, TxType::CreateNft);
            }
            TxOutput::CreateOrder(order_data) => {
                self.tx_type = merge_tx_type(self.tx_type, TxType::CreateOrder);
                let (coin_or_token_id, amount) =
                    into_coin_or_token_id_and_amount(&order_data.give)?;
                self.increase_output_totals(coin_or_token_id, amount)?;
            }
        }

        Ok(())
    }

    pub fn process_input(&mut self, inp: &TxInputWithAdditionalInfo) -> Result<(), StatusWord> {
        match inp {
            TxInputWithAdditionalInfo::Utxo(_, info) => match info {
                AdditionalUtxoInfo::UtxoWithPoolData {
                    utxo: _,
                    staker_balance,
                } => {
                    self.tx_type = merge_tx_type(self.tx_type, TxType::DecommissionStakePool);
                    self.increase_input_totals(CoinOrTokenId::Coin, *staker_balance)?;
                }
                AdditionalUtxoInfo::Utxo(utxo) => {
                    match &utxo {
                        TxOutput::Transfer(value, _)
                        | TxOutput::LockThenTransfer(value, _, _)
                        | TxOutput::Htlc(value, _) => {
                            let (coin_or_token_id, amount) =
                                into_coin_or_token_id_and_amount(value)?;
                            self.increase_input_totals(coin_or_token_id, amount)?;
                        }
                        TxOutput::Burn(_)
                        | TxOutput::ProduceBlockFromStake(_, _)
                        | TxOutput::CreateDelegationId(_, _)
                        | TxOutput::IssueFungibleToken(_)
                        | TxOutput::DataDeposit(_)
                        | TxOutput::DelegateStaking(_, _)
                        | TxOutput::CreateOrder(_) => return Err(StatusWord::TxInvalidInputUtxo),
                        TxOutput::CreateStakePool(_, data) => {
                            self.increase_input_totals(CoinOrTokenId::Coin, data.pledge)?;
                        }
                        TxOutput::IssueNft(nft_id, _, _) => {
                            self.increase_input_totals(
                                CoinOrTokenId::TokenId(*nft_id),
                                Amount::from_atoms(1),
                            )?;
                        }
                    };
                }
            },
            TxInputWithAdditionalInfo::Account(acc) => {
                self.input_command = Some(InputCommand::AccountSpending(acc.spending.clone()));
                match acc.spending {
                    AccountSpending::DelegationBalance(_, amount) => {
                        self.tx_type = merge_tx_type(self.tx_type, TxType::DelegationWithdrawal);
                        self.increase_input_totals(CoinOrTokenId::Coin, amount)?;
                    }
                }
            }
            TxInputWithAdditionalInfo::AccountCommand(_, cmd) => {
                self.input_command = Some(InputCommand::AccountCommand(cmd.clone()));
                match cmd {
                    AccountCommand::MintTokens(token_id, amount) => {
                        self.tx_type = merge_tx_type(self.tx_type, TxType::MintTokens);
                        self.increase_input_totals(CoinOrTokenId::TokenId(*token_id), *amount)?;
                    }
                    AccountCommand::ConcludeOrder(_) | AccountCommand::FillOrder(_, _, _) => {
                        return Err(StatusWord::OrdersV0NotSupported);
                    }
                    AccountCommand::UnmintTokens(_) => {
                        self.tx_type = merge_tx_type(self.tx_type, TxType::UnmintTokens);
                    }
                    AccountCommand::LockTokenSupply(_) => {
                        self.tx_type = merge_tx_type(self.tx_type, TxType::LockTokenSupply);
                    }
                    AccountCommand::FreezeToken(_, _) => {
                        self.tx_type = merge_tx_type(self.tx_type, TxType::FreezeToken);
                    }
                    AccountCommand::UnfreezeToken(_) => {
                        self.tx_type = merge_tx_type(self.tx_type, TxType::UnfreezeToken);
                    }
                    AccountCommand::ChangeTokenAuthority(_, _) => {
                        self.tx_type = merge_tx_type(self.tx_type, TxType::ChangeTokenAuthority);
                    }
                    AccountCommand::ChangeTokenMetadataUri(_, _) => {
                        self.tx_type = merge_tx_type(self.tx_type, TxType::ChangeTokenMetadataUri);
                    }
                }
            }
            TxInputWithAdditionalInfo::OrderAccountCommand(cmd, additional_info) => {
                self.input_command = Some(InputCommand::OrderCommand(
                    cmd.clone(),
                    additional_info.clone(),
                ));
                match cmd {
                    OrderAccountCommand::FillOrder(_, fill_amount) => {
                        let (fill_coin_or_token_id, asked_amount) =
                            into_coin_or_token_id_and_amount(&additional_info.initially_asked)?;
                        let (given_coin_or_token_id, given_amount) =
                            into_coin_or_token_id_and_amount(&additional_info.initially_given)?;

                        self.increase_output_totals(fill_coin_or_token_id, *fill_amount)?;

                        let atoms = given_amount
                            .into_atoms()
                            .checked_mul(fill_amount.into_atoms())
                            .ok_or(StatusWord::TxNumericOperationFail)?
                            .checked_div(asked_amount.into_atoms())
                            .ok_or(StatusWord::TxNumericOperationFail)?;
                        let amount = Amount::from_atoms(atoms);
                        self.increase_input_totals(given_coin_or_token_id, amount)?;

                        self.tx_type = merge_tx_type(self.tx_type, TxType::FillOrder);
                    }
                    OrderAccountCommand::ConcludeOrder(_) => {
                        let (coin_or_token_id, _) =
                            into_coin_or_token_id_and_amount(&additional_info.initially_asked)?;
                        self.increase_input_totals(coin_or_token_id, additional_info.ask_balance)?;

                        let (coin_or_token_id, _) =
                            into_coin_or_token_id_and_amount(&additional_info.initially_given)?;
                        self.increase_input_totals(coin_or_token_id, additional_info.give_balance)?;

                        self.tx_type = merge_tx_type(self.tx_type, TxType::ConcludeOrder);
                    }
                    OrderAccountCommand::FreezeOrder(_) => {
                        self.tx_type = merge_tx_type(self.tx_type, TxType::FreezeOrder);
                    }
                }
            }
        };

        Ok(())
    }

    fn increase_input_totals(
        &mut self,
        key: CoinOrTokenId,
        amount: Amount,
    ) -> Result<(), StatusWord> {
        let total = self
            .total_inputs
            .entry(key)
            .or_insert(Amount::from_atoms(0));
        let new_total = total
            .into_atoms()
            .checked_add(amount.into_atoms())
            .ok_or(StatusWord::TxNumericOperationFail)?;
        *total = Amount::from_atoms(new_total);
        Ok(())
    }

    fn increase_output_totals(
        &mut self,
        key: CoinOrTokenId,
        amount: Amount,
    ) -> Result<(), StatusWord> {
        let total = self
            .total_outputs
            .entry(key)
            .or_insert(Amount::from_atoms(0));
        let new_total = total
            .into_atoms()
            .checked_add(amount.into_atoms())
            .ok_or(StatusWord::TxNumericOperationFail)?;
        *total = Amount::from_atoms(new_total);
        Ok(())
    }
}

// TODO: this logic makes the tx type depend on the outputs order:
// if the outputs are Burn and Transfer, the tx type will be Burn, but if it's Transfer, Burn,
// then the tx type will be ComplexTransaction.
// See https://github.com/mintlayer/mintlayer-ledger-app/issues/21.
fn merge_tx_type(tx_type: Option<TxType>, new_type: TxType) -> Option<TxType> {
    match tx_type {
        None => Some(new_type),
        // Transfers are a lower priority (as they can be change outputs) so keep the previous type
        Some(_) if new_type == TxType::Transfer => tx_type,
        Some(_) => Some(TxType::ComplexTransaction),
    }
}

fn into_coin_or_token_id_and_amount(
    value: &OutputValue,
) -> Result<(CoinOrTokenId, Amount), StatusWord> {
    match value {
        OutputValue::Coin(amount) => Ok((CoinOrTokenId::Coin, *amount)),
        OutputValue::TokenV1(token_id, amount) => Ok((CoinOrTokenId::TokenId(*token_id), *amount)),
    }
}

#[cfg(test)]
mod tests {
    use mintlayer_messages::{AdditionalOrderInfo, AdditionalUtxoInfo, TxInputWithAdditionalInfo};
    use test_utils::prelude::*;

    use crate::{StatusWord, mlcp};

    use super::*;

    fn make_utxo_outpoint() -> mlcp::UtxoOutPoint {
        mlcp::UtxoOutPoint::new(
            mlcp::OutPointSourceId::Transaction(mlcp::Id::new(mlcp::H256::zero())),
            0,
        )
    }

    // TODO: these tests can be improved:
    // 1) Each test should better check everything (total inputs, total outputs, fees, tx type etc),
    //    even if it's only dealing with one aspect of the summary collector (e.g. tx outputs).
    // 2) More tests for fee calculation would be nice:
    //    a) non-trivial successful case;
    //    b) cases dealing with more than once currency (both successful and not), in particular
    //       the case where one currency is only present in the total outputs but not total inputs.
    // 3) Maybe something else.
    // See https://github.com/mintlayer/mintlayer-ledger-app/issues/15.

    #[test_item]
    fn test_new_and_getters() {
        let collector = TxSummaryCollector::new();
        assert!(collector.tx_type().is_none());
        assert!(collector.input_command().is_none());
        assert!(collector.total_inputs().is_empty());
        assert!(collector.total_outputs().is_empty());
    }

    #[test_item]
    fn test_process_output_transfer() {
        let mut collector = TxSummaryCollector::new();

        // Transfer Coin
        let coin_amount = mlcp::Amount::from_atoms(100);
        let out_coin = mlcp::TxOutput::Transfer(
            mlcp::OutputValue::Coin(coin_amount),
            mlcp::Destination::AnyoneCanSpend,
        );
        collector.process_output(&out_coin).unwrap();
        assert_eq!(collector.tx_type(), Some(TxType::Transfer));
        assert_eq!(
            collector.total_outputs().get(&CoinOrTokenId::Coin),
            Some(&coin_amount)
        );

        // Transfer Token
        let token_id = mlcp::Id::new(mlcp::H256::zero());
        let token_amount = mlcp::Amount::from_atoms(200);
        let out_token = mlcp::TxOutput::Transfer(
            mlcp::OutputValue::TokenV1(token_id, token_amount),
            mlcp::Destination::AnyoneCanSpend,
        );
        collector.process_output(&out_token).unwrap();
        assert_eq!(collector.tx_type(), Some(TxType::Transfer));
        assert_eq!(
            collector
                .total_outputs()
                .get(&CoinOrTokenId::TokenId(token_id)),
            Some(&token_amount)
        );
    }

    #[test_item]
    fn test_process_output_lock_then_transfer() {
        let mut collector = TxSummaryCollector::new();
        let amount = mlcp::Amount::from_atoms(150);
        let out = mlcp::TxOutput::LockThenTransfer(
            mlcp::OutputValue::Coin(amount),
            mlcp::Destination::AnyoneCanSpend,
            mlcp::OutputTimeLock::ForBlockCount(mlcp::BlocksCount(10)),
        );
        collector.process_output(&out).unwrap();
        assert_eq!(collector.tx_type(), Some(TxType::Transfer));
        assert_eq!(
            collector.total_outputs().get(&CoinOrTokenId::Coin),
            Some(&amount)
        );
    }

    #[test_item]
    fn test_process_output_burn() {
        let mut collector = TxSummaryCollector::new();
        let burn_amount = mlcp::Amount::from_atoms(50);
        let out = mlcp::TxOutput::Burn(mlcp::OutputValue::Coin(burn_amount));
        collector.process_output(&out).unwrap();
        assert_eq!(collector.tx_type(), Some(TxType::Burn));
        assert_eq!(
            collector.total_outputs().get(&CoinOrTokenId::Coin),
            Some(&burn_amount)
        );
    }

    #[test_item]
    fn test_process_output_htlc() {
        let mut collector = TxSummaryCollector::new();
        let htlc = mlcp::HashedTimelockContract {
            secret_hash: [0; 20].into(),
            spend_key: mlcp::Destination::AnyoneCanSpend,
            refund_timelock: mlcp::OutputTimeLock::ForBlockCount(mlcp::BlocksCount(10)),
            refund_key: mlcp::Destination::AnyoneCanSpend,
        };
        let htlc_amount = mlcp::Amount::from_atoms(300);
        let out = mlcp::TxOutput::Htlc(mlcp::OutputValue::Coin(htlc_amount), htlc);
        collector.process_output(&out).unwrap();
        assert_eq!(collector.tx_type(), Some(TxType::Htlc));
        assert_eq!(
            collector.total_outputs().get(&CoinOrTokenId::Coin),
            Some(&htlc_amount)
        );
    }

    #[test_item]
    fn test_process_output_create_stake_pool() {
        let mut collector = TxSummaryCollector::new();
        let pledge_amount = mlcp::Amount::from_atoms(40000);
        let data = mlcp::StakePoolData {
            pledge: pledge_amount,
            staker: mlcp::Destination::AnyoneCanSpend,
            vrf_public_key: mlcp::VrfPublicKey::Schnorrkel(mlcp::SchnorrkelPublicKey([0; 32])),
            decommission_key: mlcp::Destination::AnyoneCanSpend,
            margin_ratio_per_thousand: mlcp::PerThousand(10),
            cost_per_block: mlcp::Amount::from_atoms(0),
        };
        let out = mlcp::TxOutput::CreateStakePool(mlcp::Id::new(mlcp::H256::zero()), data);
        collector.process_output(&out).unwrap();
        assert_eq!(collector.tx_type(), Some(TxType::CreateStakePool));
        assert_eq!(
            collector.total_outputs().get(&CoinOrTokenId::Coin),
            Some(&pledge_amount)
        );
    }

    #[test_item]
    fn test_process_output_produce_block_from_stake() {
        let mut collector = TxSummaryCollector::new();
        let out = mlcp::TxOutput::ProduceBlockFromStake(
            mlcp::Destination::AnyoneCanSpend,
            mlcp::Id::new(mlcp::H256::zero()),
        );
        collector.process_output(&out).unwrap();
        // ProduceBlockFromStake is a no-op, tx_type should remain None
        assert!(collector.tx_type().is_none());
        assert!(collector.total_outputs().is_empty());
    }

    #[test_item]
    fn test_process_output_delegate_staking() {
        let mut collector = TxSummaryCollector::new();
        let delegate_amount = mlcp::Amount::from_atoms(500);
        let out =
            mlcp::TxOutput::DelegateStaking(delegate_amount, mlcp::Id::new(mlcp::H256::zero()));
        collector.process_output(&out).unwrap();
        assert_eq!(collector.tx_type(), Some(TxType::DelegateStaking));
        assert_eq!(
            collector.total_outputs().get(&CoinOrTokenId::Coin),
            Some(&delegate_amount)
        );
    }

    #[test_item]
    fn test_process_output_create_delegation_id() {
        let mut collector = TxSummaryCollector::new();
        let out = mlcp::TxOutput::CreateDelegationId(
            mlcp::Destination::AnyoneCanSpend,
            mlcp::Id::new(mlcp::H256::zero()),
        );
        collector.process_output(&out).unwrap();
        assert_eq!(collector.tx_type(), Some(TxType::CreateDelegation));
    }

    #[test_item]
    fn test_process_output_issue_fungible_token() {
        let mut collector = TxSummaryCollector::new();
        let token_issuance = mlcp::TokenIssuance::V1(mlcp::TokenIssuanceV1 {
            token_ticker: alloc::vec::Vec::new(),
            number_of_decimals: 8,
            metadata_uri: alloc::vec::Vec::new(),
            total_supply: mlcp::TokenTotalSupply::Unlimited,
            authority: mlcp::Destination::AnyoneCanSpend,
            is_freezable: mlcp::IsTokenFreezable::No,
        });
        let out = mlcp::TxOutput::IssueFungibleToken(token_issuance);
        collector.process_output(&out).unwrap();
        assert_eq!(collector.tx_type(), Some(TxType::CreateToken));
    }

    #[test_item]
    fn test_process_output_data_deposit() {
        let mut collector = TxSummaryCollector::new();
        let out = mlcp::TxOutput::DataDeposit(alloc::vec::Vec::new());
        collector.process_output(&out).unwrap();
        assert_eq!(collector.tx_type(), Some(TxType::DataDeposit));
    }

    #[test_item]
    fn test_process_output_issue_nft() {
        let mut collector = TxSummaryCollector::new();
        let nft_issuance = mlcp::NftIssuance::V0(mlcp::NftIssuanceV0 {
            creator: None,
            name: alloc::vec::Vec::new(),
            description: alloc::vec::Vec::new(),
            ticker: alloc::vec::Vec::new(),
            icon_uri: alloc::vec::Vec::new(),
            additional_metadata_uri: alloc::vec::Vec::new(),
            media_uri: alloc::vec::Vec::new(),
            media_hash: alloc::vec::Vec::new(),
        });
        let out = mlcp::TxOutput::IssueNft(
            mlcp::Id::new(mlcp::H256::zero()),
            nft_issuance,
            mlcp::Destination::AnyoneCanSpend,
        );
        collector.process_output(&out).unwrap();
        assert_eq!(collector.tx_type(), Some(TxType::CreateNft));
    }

    #[test_item]
    fn test_process_output_create_order() {
        let mut collector = TxSummaryCollector::new();
        let ask_amount = mlcp::Amount::from_atoms(100);
        let give_amount = mlcp::Amount::from_atoms(50);
        let order_data = mlcp::OrderData {
            conclude_key: mlcp::Destination::AnyoneCanSpend,
            ask: mlcp::OutputValue::Coin(ask_amount),
            give: mlcp::OutputValue::Coin(give_amount),
        };
        let out = mlcp::TxOutput::CreateOrder(order_data);
        collector.process_output(&out).unwrap();
        assert_eq!(collector.tx_type(), Some(TxType::CreateOrder));
        assert_eq!(
            collector.total_outputs().get(&CoinOrTokenId::Coin),
            Some(&give_amount)
        );
    }

    #[test_item]
    fn test_process_input_utxo_transfer() {
        let mut collector = TxSummaryCollector::new();

        let transfer_amount = mlcp::Amount::from_atoms(250);
        let inp = TxInputWithAdditionalInfo::Utxo(
            make_utxo_outpoint(),
            AdditionalUtxoInfo::Utxo(mlcp::TxOutput::Transfer(
                mlcp::OutputValue::Coin(transfer_amount),
                mlcp::Destination::AnyoneCanSpend,
            )),
        );
        collector.process_input(&inp).unwrap();
        assert_eq!(
            collector.total_inputs().get(&CoinOrTokenId::Coin),
            Some(&transfer_amount)
        );
    }

    #[test_item]
    fn test_process_input_utxo_lock_then_transfer() {
        let mut collector = TxSummaryCollector::new();

        let lock_amount = mlcp::Amount::from_atoms(120);
        let inp = TxInputWithAdditionalInfo::Utxo(
            make_utxo_outpoint(),
            AdditionalUtxoInfo::Utxo(mlcp::TxOutput::LockThenTransfer(
                mlcp::OutputValue::Coin(lock_amount),
                mlcp::Destination::AnyoneCanSpend,
                mlcp::OutputTimeLock::ForBlockCount(mlcp::BlocksCount(10)),
            )),
        );
        collector.process_input(&inp).unwrap();
        assert_eq!(
            collector.total_inputs().get(&CoinOrTokenId::Coin),
            Some(&lock_amount)
        );
    }

    #[test_item]
    fn test_process_input_utxo_htlc() {
        let mut collector = TxSummaryCollector::new();
        let htlc = mlcp::HashedTimelockContract {
            secret_hash: [0; 20].into(),
            spend_key: mlcp::Destination::AnyoneCanSpend,
            refund_timelock: mlcp::OutputTimeLock::ForBlockCount(mlcp::BlocksCount(10)),
            refund_key: mlcp::Destination::AnyoneCanSpend,
        };

        let htlc_amount = mlcp::Amount::from_atoms(80);
        let inp = TxInputWithAdditionalInfo::Utxo(
            make_utxo_outpoint(),
            AdditionalUtxoInfo::Utxo(mlcp::TxOutput::Htlc(
                mlcp::OutputValue::Coin(htlc_amount),
                htlc,
            )),
        );
        collector.process_input(&inp).unwrap();
        assert_eq!(
            collector.total_inputs().get(&CoinOrTokenId::Coin),
            Some(&htlc_amount)
        );
    }

    #[test_item]
    fn test_process_input_utxo_create_stake_pool() {
        let mut collector = TxSummaryCollector::new();
        let pledge_amount = mlcp::Amount::from_atoms(40000);
        let data = mlcp::StakePoolData {
            pledge: pledge_amount,
            staker: mlcp::Destination::AnyoneCanSpend,
            vrf_public_key: mlcp::VrfPublicKey::Schnorrkel(mlcp::SchnorrkelPublicKey([0; 32])),
            decommission_key: mlcp::Destination::AnyoneCanSpend,
            margin_ratio_per_thousand: mlcp::PerThousand(10),
            cost_per_block: mlcp::Amount::from_atoms(0),
        };

        let inp = TxInputWithAdditionalInfo::Utxo(
            make_utxo_outpoint(),
            AdditionalUtxoInfo::Utxo(mlcp::TxOutput::CreateStakePool(
                mlcp::Id::new(mlcp::H256::zero()),
                data,
            )),
        );
        collector.process_input(&inp).unwrap();
        assert_eq!(
            collector.total_inputs().get(&CoinOrTokenId::Coin),
            Some(&pledge_amount)
        );
    }

    #[test_item]
    fn test_process_input_utxo_issue_nft() {
        let mut collector = TxSummaryCollector::new();
        let nft_issuance = mlcp::NftIssuance::V0(mlcp::NftIssuanceV0 {
            creator: None,
            name: alloc::vec::Vec::new(),
            description: alloc::vec::Vec::new(),
            ticker: alloc::vec::Vec::new(),
            icon_uri: alloc::vec::Vec::new(),
            additional_metadata_uri: alloc::vec::Vec::new(),
            media_uri: alloc::vec::Vec::new(),
            media_hash: alloc::vec::Vec::new(),
        });
        let nft_id = mlcp::Id::new(mlcp::H256::zero());

        let inp = TxInputWithAdditionalInfo::Utxo(
            make_utxo_outpoint(),
            AdditionalUtxoInfo::Utxo(mlcp::TxOutput::IssueNft(
                nft_id,
                nft_issuance,
                mlcp::Destination::AnyoneCanSpend,
            )),
        );
        collector.process_input(&inp).unwrap();
        assert_eq!(
            collector
                .total_inputs()
                .get(&CoinOrTokenId::TokenId(nft_id)),
            Some(&mlcp::Amount::from_atoms(1))
        );
    }

    #[test_item]
    fn test_process_input_utxo_with_pool_data() {
        let mut collector = TxSummaryCollector::new();
        let staker_balance_amount = mlcp::Amount::from_atoms(50000);
        let inp = TxInputWithAdditionalInfo::Utxo(
            make_utxo_outpoint(),
            AdditionalUtxoInfo::UtxoWithPoolData {
                utxo: mlcp::TxOutput::ProduceBlockFromStake(
                    mlcp::Destination::AnyoneCanSpend,
                    mlcp::Id::new(mlcp::H256::zero()),
                ),
                staker_balance: staker_balance_amount,
            },
        );
        collector.process_input(&inp).unwrap();
        assert_eq!(collector.tx_type(), Some(TxType::DecommissionStakePool));
        assert_eq!(
            collector.total_inputs().get(&CoinOrTokenId::Coin),
            Some(&staker_balance_amount)
        );
    }

    #[test_item]
    fn test_process_input_account() {
        let mut collector = TxSummaryCollector::new();
        let delegation_balance = mlcp::Amount::from_atoms(700);
        let acc = mlcp::AccountOutPoint {
            nonce: mlcp::AccountNonce(0),
            spending: mlcp::AccountSpending::DelegationBalance(
                mlcp::Id::new(mlcp::H256::zero()),
                delegation_balance,
            ),
        };
        let inp = TxInputWithAdditionalInfo::Account(acc);
        collector.process_input(&inp).unwrap();
        assert_eq!(collector.tx_type(), Some(TxType::DelegationWithdrawal));
        assert_eq!(
            collector.total_inputs().get(&CoinOrTokenId::Coin),
            Some(&delegation_balance)
        );
        assert!(matches!(
            collector.input_command(),
            Some(InputCommand::AccountSpending(
                mlcp::AccountSpending::DelegationBalance(_, _)
            ))
        ));
    }

    #[test_item]
    fn test_process_input_account_command_mint() {
        let mut collector = TxSummaryCollector::new();
        let token_id = mlcp::Id::new(mlcp::H256::zero());
        let mint_amount = mlcp::Amount::from_atoms(1000);
        let inp = TxInputWithAdditionalInfo::AccountCommand(
            mlcp::AccountNonce(1),
            mlcp::AccountCommand::MintTokens(token_id, mint_amount),
        );
        collector.process_input(&inp).unwrap();
        assert_eq!(collector.tx_type(), Some(TxType::MintTokens));
        assert_eq!(
            collector
                .total_inputs()
                .get(&CoinOrTokenId::TokenId(token_id)),
            Some(&mint_amount)
        );
        assert!(matches!(
            collector.input_command(),
            Some(InputCommand::AccountCommand(
                mlcp::AccountCommand::MintTokens(_, _)
            ))
        ));
    }

    #[test_item]
    fn test_process_input_account_command_unmint() {
        let mut collector = TxSummaryCollector::new();
        let token_id = mlcp::Id::new(mlcp::H256::zero());
        let inp = TxInputWithAdditionalInfo::AccountCommand(
            mlcp::AccountNonce(2),
            mlcp::AccountCommand::UnmintTokens(token_id),
        );
        collector.process_input(&inp).unwrap();
        assert_eq!(collector.tx_type(), Some(TxType::UnmintTokens));
    }

    #[test_item]
    fn test_process_input_account_command_lock_token_supply() {
        let mut collector = TxSummaryCollector::new();
        let token_id = mlcp::Id::new(mlcp::H256::zero());
        let inp = TxInputWithAdditionalInfo::AccountCommand(
            mlcp::AccountNonce(3),
            mlcp::AccountCommand::LockTokenSupply(token_id),
        );
        collector.process_input(&inp).unwrap();
        assert_eq!(collector.tx_type(), Some(TxType::LockTokenSupply));
    }

    #[test_item]
    fn test_process_input_account_command_freeze_token() {
        let mut collector = TxSummaryCollector::new();
        let token_id = mlcp::Id::new(mlcp::H256::zero());
        let inp = TxInputWithAdditionalInfo::AccountCommand(
            mlcp::AccountNonce(4),
            mlcp::AccountCommand::FreezeToken(token_id, mlcp::IsTokenUnfreezable::Yes),
        );
        collector.process_input(&inp).unwrap();
        assert_eq!(collector.tx_type(), Some(TxType::FreezeToken));
    }

    #[test_item]
    fn test_process_input_account_command_unfreeze_token() {
        let mut collector = TxSummaryCollector::new();
        let token_id = mlcp::Id::new(mlcp::H256::zero());
        let inp = TxInputWithAdditionalInfo::AccountCommand(
            mlcp::AccountNonce(5),
            mlcp::AccountCommand::UnfreezeToken(token_id),
        );
        collector.process_input(&inp).unwrap();
        assert_eq!(collector.tx_type(), Some(TxType::UnfreezeToken));
    }

    #[test_item]
    fn test_process_input_account_command_change_authority() {
        let mut collector = TxSummaryCollector::new();
        let token_id = mlcp::Id::new(mlcp::H256::zero());
        let inp = TxInputWithAdditionalInfo::AccountCommand(
            mlcp::AccountNonce(6),
            mlcp::AccountCommand::ChangeTokenAuthority(token_id, mlcp::Destination::AnyoneCanSpend),
        );
        collector.process_input(&inp).unwrap();
        assert_eq!(collector.tx_type(), Some(TxType::ChangeTokenAuthority));
    }

    #[test_item]
    fn test_process_input_account_command_change_metadata_uri() {
        let mut collector = TxSummaryCollector::new();
        let token_id = mlcp::Id::new(mlcp::H256::zero());
        let inp = TxInputWithAdditionalInfo::AccountCommand(
            mlcp::AccountNonce(7),
            mlcp::AccountCommand::ChangeTokenMetadataUri(token_id, alloc::vec::Vec::new()),
        );
        collector.process_input(&inp).unwrap();
        assert_eq!(collector.tx_type(), Some(TxType::ChangeTokenMetadataUri));
    }

    #[test_item]
    fn test_process_input_order_command_fill() {
        let mut collector = TxSummaryCollector::new();
        let order_id = mlcp::Id::new(mlcp::H256::zero());
        let fill_amount = mlcp::Amount::from_atoms(10);
        let initially_asked = mlcp::Amount::from_atoms(100);
        let initially_given = mlcp::Amount::from_atoms(200);

        let additional_info = AdditionalOrderInfo {
            initially_asked: mlcp::OutputValue::Coin(initially_asked),
            initially_given: mlcp::OutputValue::Coin(initially_given),
            ask_balance: mlcp::Amount::from_atoms(0),
            give_balance: mlcp::Amount::from_atoms(0),
        };
        let inp = TxInputWithAdditionalInfo::OrderAccountCommand(
            mlcp::OrderAccountCommand::FillOrder(order_id, fill_amount),
            additional_info,
        );
        collector.process_input(&inp).unwrap();
        assert_eq!(collector.tx_type(), Some(TxType::FillOrder));
        // Fill order output totals increases by fill_amount (10 coins)
        assert_eq!(
            collector.total_outputs().get(&CoinOrTokenId::Coin),
            Some(&fill_amount)
        );
        // Fill order input totals increases by (given_amount * fill_amount / asked_amount)
        // 200 * 10 / 100 = 20
        let expected_input_amount = Amount::from_atoms(
            (initially_given.into_atoms() * fill_amount.into_atoms())
                / initially_asked.into_atoms(),
        );
        assert_eq!(
            collector.total_inputs().get(&CoinOrTokenId::Coin),
            Some(&expected_input_amount)
        );
        assert!(matches!(
            collector.input_command(),
            Some(InputCommand::OrderCommand(
                mlcp::OrderAccountCommand::FillOrder(_, _),
                AdditionalOrderInfo { .. }
            ))
        ));
    }

    #[test_item]
    fn test_process_input_order_command_conclude() {
        let mut collector = TxSummaryCollector::new();
        let order_id = mlcp::Id::new(mlcp::H256::zero());
        let ask_balance = mlcp::Amount::from_atoms(30);
        let give_balance = mlcp::Amount::from_atoms(60);
        let token_id = mlcp::Id::new(mlcp::H256::zero());
        let additional_info = AdditionalOrderInfo {
            initially_asked: mlcp::OutputValue::Coin(mlcp::Amount::from_atoms(100)),
            initially_given: mlcp::OutputValue::TokenV1(token_id, mlcp::Amount::from_atoms(200)),
            ask_balance,
            give_balance,
        };
        let inp = TxInputWithAdditionalInfo::OrderAccountCommand(
            mlcp::OrderAccountCommand::ConcludeOrder(order_id),
            additional_info,
        );
        collector.process_input(&inp).unwrap();
        assert_eq!(collector.tx_type(), Some(TxType::ConcludeOrder));
        // Conclude order increases inputs by ask_balance and give_balance
        assert_eq!(
            collector.total_inputs().get(&CoinOrTokenId::Coin),
            Some(&ask_balance)
        );
        assert_eq!(
            collector
                .total_inputs()
                .get(&CoinOrTokenId::TokenId(token_id)),
            Some(&give_balance)
        );
    }

    #[test_item]
    fn test_process_input_order_command_freeze() {
        let mut collector = TxSummaryCollector::new();
        let order_id = mlcp::Id::new(mlcp::H256::zero());
        let additional_info = AdditionalOrderInfo {
            initially_asked: mlcp::OutputValue::Coin(mlcp::Amount::from_atoms(100)),
            initially_given: mlcp::OutputValue::Coin(mlcp::Amount::from_atoms(200)),
            ask_balance: mlcp::Amount::from_atoms(0),
            give_balance: mlcp::Amount::from_atoms(0),
        };
        let inp = TxInputWithAdditionalInfo::OrderAccountCommand(
            mlcp::OrderAccountCommand::FreezeOrder(order_id),
            additional_info,
        );
        collector.process_input(&inp).unwrap();
        assert_eq!(collector.tx_type(), Some(TxType::FreezeOrder));
    }

    #[test_item]
    fn test_process_input_errors() {
        let mut collector = TxSummaryCollector::new();

        // 1. Burn output as input UTXO is invalid
        let inp_burn = TxInputWithAdditionalInfo::Utxo(
            make_utxo_outpoint(),
            AdditionalUtxoInfo::Utxo(mlcp::TxOutput::Burn(mlcp::OutputValue::Coin(
                mlcp::Amount::from_atoms(100),
            ))),
        );
        assert_eq!(
            collector.process_input(&inp_burn).unwrap_err(),
            StatusWord::TxInvalidInputUtxo
        );

        // 2. OrdersV0NotSupported in AccountCommand
        let inp_conclude_v0 = TxInputWithAdditionalInfo::AccountCommand(
            mlcp::AccountNonce(8),
            mlcp::AccountCommand::ConcludeOrder(mlcp::Id::new(mlcp::H256::zero())),
        );
        assert_eq!(
            collector.process_input(&inp_conclude_v0).unwrap_err(),
            StatusWord::OrdersV0NotSupported
        );

        let inp_fill_v0 = TxInputWithAdditionalInfo::AccountCommand(
            mlcp::AccountNonce(9),
            mlcp::AccountCommand::FillOrder(
                mlcp::Id::new(mlcp::H256::zero()),
                mlcp::Amount::from_atoms(100),
                mlcp::Destination::AnyoneCanSpend,
            ),
        );
        assert_eq!(
            collector.process_input(&inp_fill_v0).unwrap_err(),
            StatusWord::OrdersV0NotSupported
        );
    }

    #[test_item]
    fn test_tx_type_merging() {
        assert_eq!(
            merge_tx_type(None, TxType::Transfer),
            Some(TxType::Transfer)
        );
        assert_eq!(
            merge_tx_type(Some(TxType::Transfer), TxType::Transfer),
            Some(TxType::Transfer)
        );
        assert_eq!(
            merge_tx_type(Some(TxType::Burn), TxType::Transfer),
            Some(TxType::Burn)
        );
        assert_eq!(
            merge_tx_type(Some(TxType::Burn), TxType::Htlc),
            Some(TxType::ComplexTransaction)
        );
    }

    #[test_item]
    fn test_fees_calculation_overflow_and_underflow() {
        let mut collector = TxSummaryCollector::new();

        // Underflow: out (120) > inp (100)
        collector
            .total_inputs
            .insert(CoinOrTokenId::Coin, mlcp::Amount::from_atoms(100));
        collector
            .total_outputs
            .insert(CoinOrTokenId::Coin, mlcp::Amount::from_atoms(120));
        let fees_res = collector.fees_iter().next().unwrap();
        assert_eq!(fees_res.unwrap_err(), StatusWord::TxFeeUnderflow);

        // Numeric overflow in increase_input_totals
        let mut collector = TxSummaryCollector::new();
        collector
            .increase_input_totals(CoinOrTokenId::Coin, mlcp::Amount::from_atoms(u128::MAX))
            .unwrap();
        assert_eq!(
            collector
                .increase_input_totals(CoinOrTokenId::Coin, mlcp::Amount::from_atoms(1))
                .unwrap_err(),
            StatusWord::TxNumericOperationFail
        );

        // Numeric overflow in increase_output_totals
        let mut collector = TxSummaryCollector::new();
        collector
            .increase_output_totals(CoinOrTokenId::Coin, mlcp::Amount::from_atoms(u128::MAX))
            .unwrap();
        assert_eq!(
            collector
                .increase_output_totals(CoinOrTokenId::Coin, mlcp::Amount::from_atoms(1))
                .unwrap_err(),
            StatusWord::TxNumericOperationFail
        );
    }
}
